//! SSH tunnel connections (issue #11): a local port forward through a
//! bastion so pgcove can reach a database bound to localhost on a remote
//! box. Auth + host-key checking follows the pattern already working in the
//! hopline sibling project's `ssh.rs`, trimmed to what a plain local
//! forward needs — no shell/PTY, and (for now) no jump-host support.

use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use russh::client::{self, Handle};
use russh::keys::agent::client::{AgentClient, AgentStream};
use russh::keys::PrivateKeyWithHashAlg;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::connections::{SshAuth, SshTunnelConfig};
use crate::known_hosts::{self, Verdict};

/// How long we'll wait for the TCP connect + SSH handshake. Auth (which can
/// involve reading a passphrase-protected key) isn't covered by this.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);
const KEEPALIVE_MAX_MISSED: usize = 3;

struct ClientHandler {
    host: String,
    port: u16,
    known_hosts_dir: PathBuf,
}

#[derive(Debug)]
enum HandlerError {
    Ssh(russh::Error),
    HostKeyChanged { host: String, port: u16 },
    KnownHosts(String),
}

impl std::fmt::Display for HandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandlerError::Ssh(e) => write!(f, "{e}"),
            HandlerError::HostKeyChanged { host, port } => write!(
                f,
                "host key for {host}:{port} does not match the key pgcove trusted before. \
                 This could mean the server was reinstalled, or that someone is intercepting \
                 the connection — refusing to connect. If you're sure this is expected, remove \
                 the old entry from pgcove's known_hosts file."
            ),
            HandlerError::KnownHosts(msg) => write!(f, "known_hosts check failed: {msg}"),
        }
    }
}

impl std::error::Error for HandlerError {}

impl From<russh::Error> for HandlerError {
    fn from(e: russh::Error) -> Self {
        HandlerError::Ssh(e)
    }
}

impl client::Handler for ClientHandler {
    type Error = HandlerError;

    async fn check_server_key(
        &mut self,
        key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        match known_hosts::verify(&self.known_hosts_dir, &self.host, self.port, key)
            .map_err(HandlerError::KnownHosts)?
        {
            Verdict::Trusted => Ok(true),
            Verdict::New => {
                known_hosts::trust(&self.known_hosts_dir, &self.host, self.port, key)
                    .map_err(HandlerError::KnownHosts)?;
                Ok(true)
            }
            Verdict::Changed => Err(HandlerError::HostKeyChanged {
                host: self.host.clone(),
                port: self.port,
            }),
        }
    }
}

fn describe_handler_error(e: HandlerError, host: &str, port: u16) -> String {
    match e {
        HandlerError::HostKeyChanged { .. } | HandlerError::KnownHosts(_) => e.to_string(),
        HandlerError::Ssh(inner) => format!("could not connect to {host}:{port}: {inner}"),
    }
}

fn client_config() -> Arc<client::Config> {
    Arc::new(client::Config {
        keepalive_interval: Some(KEEPALIVE_INTERVAL),
        keepalive_max: KEEPALIVE_MAX_MISSED,
        ..Default::default()
    })
}

/// `.dynamic()` erases the platform-specific stream type (Unix socket vs.
/// Windows named pipe) so the caller doesn't need `cfg`.
type DynAgentClient = AgentClient<Box<dyn AgentStream + Send + Unpin + 'static>>;

/// Connects to the platform's running agent: ssh-agent via `SSH_AUTH_SOCK`
/// on Unix, the Windows OpenSSH agent's named pipe on Windows (falling back
/// to Pageant if that's not running — both are common). Ported from
/// hopline's `ssh::connect_agent`.
#[cfg(unix)]
async fn connect_agent() -> Result<DynAgentClient, String> {
    AgentClient::connect_env()
        .await
        .map(AgentClient::dynamic)
        .map_err(|_| {
            "could not reach an SSH agent — check that one is running and SSH_AUTH_SOCK is set"
                .to_string()
        })
}

#[cfg(windows)]
async fn connect_agent() -> Result<DynAgentClient, String> {
    if let Ok(client) = AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent").await {
        return Ok(client.dynamic());
    }
    Ok(AgentClient::connect_pageant().await.dynamic())
}

/// Authenticates using whichever of the agent's loaded identities the
/// server accepts first. A rejected key just moves on to the next one; a
/// failure signing with the agent itself (a broken pipe, Pageant closing
/// mid-request) aborts the loop instead of silently trying the rest.
///
/// Explicitly boxed: `authenticate_publickey_with`'s `S: auth::Signer` bound
/// (RPITIT under the hood) otherwise produces a "Send is not general
/// enough" error from tauri::command's macro-generated Future once this is
/// called from an async command — a known rustc/async-trait HRTB inference
/// limitation, not a real soundness issue. Boxing gives the compiler a
/// concrete, already-erased Future type to reason about instead. Same fix
/// hopline's `ssh::authenticate_via_agent` uses.
fn authenticate_via_agent<'a>(
    session: &'a mut Handle<ClientHandler>,
    user: &'a str,
) -> Pin<Box<dyn Future<Output = Result<russh::client::AuthResult, String>> + Send + 'a>> {
    Box::pin(async move {
        let mut agent = connect_agent().await?;
        let identities = agent
            .request_identities()
            .await
            .map_err(|_| "could not list identities from the SSH agent".to_string())?;
        if identities.is_empty() {
            return Err(
                "the SSH agent has no keys loaded — add one (ssh-add, or load it into Pageant) \
                 and try again"
                    .to_string(),
            );
        }
        let best_hash = session
            .best_supported_rsa_hash()
            .await
            .map_err(|e| e.to_string())?
            .flatten();
        for key in identities {
            let attempt: Pin<
                Box<
                    dyn Future<Output = Result<russh::client::AuthResult, russh::AgentAuthError>>
                        + Send
                        + '_,
                >,
            > = Box::pin(session.authenticate_publickey_with(
                user.to_string(),
                key,
                best_hash,
                &mut agent,
            ));
            match attempt.await {
                Ok(result) if result.success() => return Ok(result),
                Ok(_) => continue,
                Err(_) => {
                    return Err(
                        "the SSH agent failed to sign the authentication request".to_string()
                    )
                }
            }
        }
        Err(format!(
            "could not sign in as \"{user}\" — none of the agent's keys were accepted"
        ))
    })
}

async fn connect_and_authenticate(
    cfg: &SshTunnelConfig,
    secret: &str,
    known_hosts_dir: &Path,
) -> Result<Handle<ClientHandler>, String> {
    let handler = ClientHandler {
        host: cfg.host.clone(),
        port: cfg.port,
        known_hosts_dir: known_hosts_dir.to_path_buf(),
    };
    let mut session = match tokio::time::timeout(
        CONNECT_TIMEOUT,
        client::connect(client_config(), (cfg.host.as_str(), cfg.port), handler),
    )
    .await
    {
        Err(_elapsed) => {
            return Err(format!(
                "connecting to {}:{} timed out after {}s",
                cfg.host,
                cfg.port,
                CONNECT_TIMEOUT.as_secs()
            ))
        }
        Ok(Err(e)) => return Err(describe_handler_error(e, &cfg.host, cfg.port)),
        Ok(Ok(session)) => session,
    };

    let authed = match &cfg.auth {
        SshAuth::Agent => authenticate_via_agent(&mut session, &cfg.user).await?,
        SshAuth::Key { key_path } => {
            let passphrase = if secret.is_empty() {
                None
            } else {
                Some(secret)
            };
            let key = russh::keys::load_secret_key(key_path, passphrase)
                .map_err(|e| format!("could not load the private key: {e}"))?;
            let best_hash = session
                .best_supported_rsa_hash()
                .await
                .map_err(|e| e.to_string())?
                .flatten();
            session
                .authenticate_publickey(
                    &cfg.user,
                    PrivateKeyWithHashAlg::new(Arc::new(key), best_hash),
                )
                .await
                .map_err(|e| e.to_string())?
        }
        SshAuth::Password => session
            .authenticate_password(&cfg.user, secret)
            .await
            .map_err(|e| e.to_string())?,
    };
    if !authed.success() {
        return Err(format!(
            "could not sign in as \"{}\" on {}:{} — check the password, passphrase, or key file",
            cfg.user, cfg.host, cfg.port
        ));
    }
    Ok(session)
}

/// Pumps bytes bidirectionally between a local TCP connection and an SSH
/// channel until either side closes. Half-duplex aware: a peer that writes
/// then immediately closes its write side must not have its last
/// `channel.data()` discarded just because the local read side stopped —
/// the channel keeps draining until the far end closes it too.
async fn bridge_tcp_and_channel(
    stream: tokio::net::TcpStream,
    mut channel: russh::Channel<client::Msg>,
) {
    let (mut read_half, mut write_half) = stream.into_split();
    let mut buf = [0u8; 8192];
    let mut local_done = false;
    loop {
        if local_done {
            match channel.wait().await {
                Some(russh::ChannelMsg::Data { data }) => {
                    if write_half.write_all(&data).await.is_err() {
                        break;
                    }
                }
                Some(russh::ChannelMsg::Eof)
                | Some(russh::ChannelMsg::ExitStatus { .. })
                | None => break,
                _ => {}
            }
            continue;
        }
        tokio::select! {
            n = read_half.read(&mut buf) => {
                match n {
                    Ok(0) | Err(_) => {
                        let _ = channel.eof().await;
                        local_done = true;
                    }
                    Ok(n) => { if channel.data(&buf[..n]).await.is_err() { break; } }
                }
            }
            msg = channel.wait() => {
                match msg {
                    Some(russh::ChannelMsg::Data { data }) => {
                        if write_half.write_all(&data).await.is_err() { break; }
                    }
                    Some(russh::ChannelMsg::Eof) | Some(russh::ChannelMsg::ExitStatus { .. }) | None => break,
                    _ => {}
                }
            }
        }
    }
}

/// A running tunnel: a local listener on `127.0.0.1:local_port` bridging
/// each accepted connection to `target_host:target_port` through the SSH
/// session. Dropping it aborts the listener and closes the SSH connection.
pub struct TunnelHandle {
    pub local_port: u16,
    listener_task: tauri::async_runtime::JoinHandle<()>,
    _ssh_handle: Arc<Handle<ClientHandler>>,
}

impl Drop for TunnelHandle {
    fn drop(&mut self) {
        self.listener_task.abort();
    }
}

/// Opens a local port forward through the bastion described by `cfg` to
/// `target_host:target_port` (the DB's address as reachable *from the
/// bastion*, e.g. `127.0.0.1:5432` for a DB bound to localhost on that box).
pub async fn start(
    cfg: &SshTunnelConfig,
    secret: &str,
    known_hosts_dir: &Path,
    target_host: String,
    target_port: u16,
) -> Result<TunnelHandle, String> {
    let handle = Arc::new(connect_and_authenticate(cfg, secret, known_hosts_dir).await?);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .map_err(|e| format!("could not open a local port for the tunnel: {e}"))?;
    let local_port = listener.local_addr().map_err(|e| e.to_string())?.port();

    let accept_handle = handle.clone();
    let listener_task = tauri::async_runtime::spawn(async move {
        loop {
            let (stream, _addr) = match listener.accept().await {
                Ok(v) => v,
                Err(_) => break,
            };
            let channel = accept_handle
                .channel_open_direct_tcpip(target_host.clone(), target_port.into(), "127.0.0.1", 0)
                .await;
            if let Ok(channel) = channel {
                tauri::async_runtime::spawn(bridge_tcp_and_channel(stream, channel));
            }
        }
    });

    Ok(TunnelHandle {
        local_port,
        listener_task,
        _ssh_handle: handle,
    })
}

/// App state: active tunnels keyed by connection id, kept alive across
/// commands (each command reconnects a fresh DB pool, but reusing the SSH
/// tunnel avoids a fresh handshake per query). Torn down on delete/edit —
/// see `commands::delete_connection` / `commands::save_connection`.
#[derive(Default)]
pub struct SshTunnels(pub Mutex<HashMap<String, Arc<TunnelHandle>>>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_key_changed_message_names_host_and_port_and_known_hosts() {
        let e = HandlerError::HostKeyChanged {
            host: "bastion".into(),
            port: 2222,
        };
        let msg = describe_handler_error(e, "bastion", 2222);
        assert!(msg.contains("bastion:2222"));
        assert!(msg.to_lowercase().contains("known_hosts"));
    }
}
