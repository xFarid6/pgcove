//! Saved database connections. Non-secret fields live in a JSON file in the
//! app config dir; the password (or Supabase service key) lives only in the
//! OS keyring — never on disk, never logged.
//!
//! Same pattern as proxmox-desktop/dockshell `connections.rs`; extraction
//! into a shared crate is issue #14.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const KEYRING_SERVICE: &str = "pgcove";

/// Which wire protocol/driver a connection uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DbKind {
    #[default]
    Postgres,
    Sqlite,
}

/// One saved connection = one database. For Supabase the SQL side is still
/// the project's Postgres (direct `db.<project-ref>.supabase.co`, or a pooler
/// host like `aws-0-<region>.pooler.supabase.com` with user
/// `postgres.<project-ref>`) — `supabase_url` additionally marks it as a
/// Supabase project so the HTTP APIs (issue #5) can be reached alongside it.
///
/// For `DbKind::Sqlite`, only `database` is meaningful — it holds the file
/// path (or `:memory:`); `host`/`port`/`user` are ignored and the password
/// is never written to the keyring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    pub id: String,
    pub name: String,
    /// Defaults to Postgres so existing `connections.json` entries (written
    /// before SQLite support existed) still deserialize.
    #[serde(default)]
    pub kind: DbKind,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub database: String,
    /// `https://<project-ref>.supabase.co` when this connection is a Supabase
    /// project. A separate `DbKind` would be wrong: the driver is still
    /// Postgres, this only says "there is also a Supabase HTTP API here". The
    /// matching service-role key lives in the keyring, never in this file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supabase_url: Option<String>,
    /// SSH tunnel (issue #11): when set, `db.rs` opens a local port forward
    /// through this bastion before pooling and connects to it instead of
    /// `host`/`port` directly. `host`/`port` above stay the DB's address as
    /// reachable *from the bastion* (often `127.0.0.1:5432` for a DB bound
    /// to localhost on a remote box).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh_tunnel: Option<SshTunnelConfig>,
}

/// Non-secret half of an SSH tunnel. The key passphrase or SSH password (for
/// `SshAuth::Password`) lives in the keyring, alongside the DB password —
/// never in `connections.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTunnelConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: SshAuth,
}

/// Which SSH auth method to use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "method")]
pub enum SshAuth {
    /// Whichever identity the platform's running ssh-agent (or, on Windows,
    /// Pageant) offers first that the server accepts — no secret needed.
    Agent,
    /// `key_path` is a private key file; its passphrase (if any, may be
    /// empty) lives in the keyring.
    Key { key_path: String },
    /// The keyring holds the SSH password.
    Password,
}

fn store_file(dir: &Path) -> PathBuf {
    dir.join("connections.json")
}

pub fn load(dir: &Path) -> Result<Vec<ConnectionInfo>, String> {
    let path = store_file(dir);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn save_all(dir: &Path, conns: &[ConnectionInfo]) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let raw = serde_json::to_string_pretty(conns).map_err(|e| e.to_string())?;
    fs::write(store_file(dir), raw).map_err(|e| e.to_string())
}

pub fn get(dir: &Path, id: &str) -> Result<ConnectionInfo, String> {
    load(dir)?
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("unknown connection: {id}"))
}

fn secret_entry(id: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, id).map_err(|e| e.to_string())
}

/// Keyring account for a connection's Supabase service-role key. Same service
/// and same `keyring::Entry` mechanism as the password — only the account
/// name differs, so one connection can hold both secrets.
fn service_key_account(id: &str) -> String {
    format!("{id}#supabase-service-key")
}

fn service_key_entry(id: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, &service_key_account(id)).map_err(|e| e.to_string())
}

/// Keyring account for a connection's SSH secret (key passphrase or SSH
/// password) — same one-account-per-secret pattern as `service_key_account`.
fn ssh_secret_account(id: &str) -> String {
    format!("{id}#ssh-secret")
}

fn ssh_secret_entry(id: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, &ssh_secret_account(id)).map_err(|e| e.to_string())
}

pub fn get_password(id: &str) -> Result<String, String> {
    secret_entry(id)?.get_password().map_err(|e| e.to_string())
}

pub fn get_service_key(id: &str) -> Result<String, String> {
    service_key_entry(id)?
        .get_password()
        .map_err(|e| e.to_string())
}

pub fn get_ssh_secret(id: &str) -> Result<String, String> {
    ssh_secret_entry(id)?
        .get_password()
        .map_err(|e| e.to_string())
}

/// Upsert a connection; `password`, `service_key` and `ssh_secret` are
/// written to the keyring when provided (add, or edit that changes them).
pub fn save(
    dir: &Path,
    info: ConnectionInfo,
    password: Option<String>,
    service_key: Option<String>,
    ssh_secret: Option<String>,
) -> Result<(), String> {
    if let Some(p) = password {
        secret_entry(&info.id)?
            .set_password(&p)
            .map_err(|e| e.to_string())?;
    }
    if let Some(k) = service_key {
        service_key_entry(&info.id)?
            .set_password(&k)
            .map_err(|e| e.to_string())?;
    }
    if let Some(s) = ssh_secret {
        ssh_secret_entry(&info.id)?
            .set_password(&s)
            .map_err(|e| e.to_string())?;
    }
    let mut conns = load(dir)?;
    match conns.iter_mut().find(|c| c.id == info.id) {
        Some(existing) => *existing = info,
        None => conns.push(info),
    }
    save_all(dir, &conns)
}

pub fn delete(dir: &Path, id: &str) -> Result<(), String> {
    // Best effort — the entries may already be gone.
    if let Ok(entry) = secret_entry(id) {
        let _ = entry.delete_credential();
    }
    if let Ok(entry) = service_key_entry(id) {
        let _ = entry.delete_credential();
    }
    if let Ok(entry) = ssh_secret_entry(id) {
        let _ = entry.delete_credential();
    }
    let mut conns = load(dir)?;
    conns.retain(|c| c.id != id);
    save_all(dir, &conns)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn(id: &str) -> ConnectionInfo {
        ConnectionInfo {
            id: id.into(),
            name: format!("db {id}"),
            kind: DbKind::Postgres,
            host: "localhost".into(),
            port: 5432,
            user: "postgres".into(),
            database: "postgres".into(),
            supabase_url: None,
            ssh_tunnel: None,
        }
    }

    #[test]
    fn save_load_delete_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()).unwrap(), vec![]);

        save(dir.path(), conn("a"), None, None, None).unwrap();
        save(dir.path(), conn("b"), None, None, None).unwrap();
        assert_eq!(load(dir.path()).unwrap().len(), 2);

        // Upsert replaces, not duplicates.
        let mut edited = conn("a");
        edited.name = "renamed".into();
        save(dir.path(), edited, None, None, None).unwrap();
        assert_eq!(load(dir.path()).unwrap().len(), 2);
        assert_eq!(get(dir.path(), "a").unwrap().name, "renamed");

        delete(dir.path(), "a").unwrap();
        assert!(get(dir.path(), "a").is_err());
    }

    #[test]
    fn supabase_url_roundtrips_and_stays_optional() {
        let dir = tempfile::tempdir().unwrap();
        let mut sb = conn("sb");
        sb.supabase_url = Some("https://abcdefgh.supabase.co".into());
        save(dir.path(), sb, None, None, None).unwrap();
        save(dir.path(), conn("plain"), None, None, None).unwrap();

        assert_eq!(
            get(dir.path(), "sb").unwrap().supabase_url.as_deref(),
            Some("https://abcdefgh.supabase.co")
        );
        assert_eq!(get(dir.path(), "plain").unwrap().supabase_url, None);
        // Non-Supabase connections keep the field out of connections.json.
        let raw = fs::read_to_string(store_file(dir.path())).unwrap();
        assert_eq!(raw.matches("supabaseUrl").count(), 1);
    }

    #[test]
    fn pre_supabase_connections_json_still_loads() {
        // A file written before this field existed must not fail to parse.
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path()).unwrap();
        fs::write(
            store_file(dir.path()),
            r#"[{"id":"old","name":"old","host":"h","port":5432,
                 "user":"postgres","database":"postgres"}]"#,
        )
        .unwrap();
        let old = get(dir.path(), "old").unwrap();
        assert_eq!(old.kind, DbKind::Postgres);
        assert_eq!(old.supabase_url, None);
        assert_eq!(old.ssh_tunnel, None);
    }

    #[test]
    fn ssh_tunnel_roundtrips_and_stays_optional() {
        let dir = tempfile::tempdir().unwrap();
        let mut tunneled = conn("t");
        tunneled.host = "127.0.0.1".into();
        tunneled.ssh_tunnel = Some(SshTunnelConfig {
            host: "bastion.example.com".into(),
            port: 22,
            user: "deploy".into(),
            auth: SshAuth::Key {
                key_path: "/home/me/.ssh/id_ed25519".into(),
            },
        });
        save(dir.path(), tunneled, None, None, None).unwrap();
        save(dir.path(), conn("plain"), None, None, None).unwrap();

        let loaded = get(dir.path(), "t").unwrap().ssh_tunnel.unwrap();
        assert_eq!(loaded.host, "bastion.example.com");
        assert_eq!(
            loaded.auth,
            SshAuth::Key {
                key_path: "/home/me/.ssh/id_ed25519".into()
            }
        );
        assert_eq!(get(dir.path(), "plain").unwrap().ssh_tunnel, None);
        // Non-tunneled connections keep the field out of connections.json.
        let raw = fs::read_to_string(store_file(dir.path())).unwrap();
        assert_eq!(raw.matches("sshTunnel").count(), 1);
    }

    #[test]
    fn ssh_auth_agent_roundtrips_with_no_extra_fields() {
        let dir = tempfile::tempdir().unwrap();
        let mut tunneled = conn("t");
        tunneled.ssh_tunnel = Some(SshTunnelConfig {
            host: "bastion.example.com".into(),
            port: 22,
            user: "deploy".into(),
            auth: SshAuth::Agent,
        });
        save(dir.path(), tunneled, None, None, None).unwrap();
        assert_eq!(
            get(dir.path(), "t").unwrap().ssh_tunnel.unwrap().auth,
            SshAuth::Agent
        );
    }

    #[test]
    fn service_key_uses_a_distinct_keyring_account() {
        assert_ne!(service_key_account("abc"), "abc");
        assert!(service_key_account("abc").starts_with("abc"));
    }

    #[test]
    fn ssh_secret_uses_a_distinct_keyring_account() {
        assert_ne!(ssh_secret_account("abc"), "abc");
        assert!(ssh_secret_account("abc").starts_with("abc"));
        assert_ne!(ssh_secret_account("abc"), service_key_account("abc"));
    }

    // Real OS keyring roundtrip. Ignored in CI (headless ubuntu has no Secret
    // Service); run locally with `cargo test -- --ignored`.
    #[test]
    #[ignore = "requires a real OS keyring; run locally with --ignored"]
    fn keyring_password_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let id = "pgcove-test-keyring";
        save(dir.path(), conn(id), Some("s3cret".into()), None, None).unwrap();
        assert_eq!(get_password(id).unwrap(), "s3cret");
        delete(dir.path(), id).unwrap();
        assert!(get_password(id).is_err());
    }

    // Password and service key must coexist for one connection id.
    #[test]
    #[ignore = "requires a real OS keyring; run locally with --ignored"]
    fn keyring_service_key_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let id = "pgcove-test-keyring-supabase";
        save(
            dir.path(),
            conn(id),
            Some("s3cret".into()),
            Some("service-role-key".into()),
            None,
        )
        .unwrap();
        assert_eq!(get_password(id).unwrap(), "s3cret");
        assert_eq!(get_service_key(id).unwrap(), "service-role-key");
        delete(dir.path(), id).unwrap();
        assert!(get_service_key(id).is_err());
    }

    // DB password and SSH secret must coexist for one connection id.
    #[test]
    #[ignore = "requires a real OS keyring; run locally with --ignored"]
    fn keyring_ssh_secret_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let id = "pgcove-test-keyring-ssh";
        save(
            dir.path(),
            conn(id),
            Some("s3cret".into()),
            None,
            Some("ssh-passphrase".into()),
        )
        .unwrap();
        assert_eq!(get_password(id).unwrap(), "s3cret");
        assert_eq!(get_ssh_secret(id).unwrap(), "ssh-passphrase");
        delete(dir.path(), id).unwrap();
        assert!(get_ssh_secret(id).is_err());
    }
}
