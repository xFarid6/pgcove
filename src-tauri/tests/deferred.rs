//! Deferred-feature test surface. Each stub names the GitHub issue that will
//! implement it — visible via `cargo test -- --ignored --list`.

// Issue #5 (Supabase project connections) shipped: project info and storage
// bucket listing over the project URL + service-role key are covered for real
// by the unit tests in src/supabase.rs (URL/header construction and response
// parsing) plus the env-gated end-to-end test below. What is left deferred is
// only the part a service-role key genuinely cannot do — see src/supabase.rs.
#[tokio::test]
#[ignore = "requires a live Supabase project — set PGCOVE_TEST_SUPABASE_URL and PGCOVE_TEST_SUPABASE_KEY"]
async fn fetches_project_info_and_buckets_from_a_live_project() {
    let url = std::env::var("PGCOVE_TEST_SUPABASE_URL")
        .expect("set PGCOVE_TEST_SUPABASE_URL=https://<ref>.supabase.co");
    let key = std::env::var("PGCOVE_TEST_SUPABASE_KEY")
        .expect("set PGCOVE_TEST_SUPABASE_KEY=<service-role key>");
    let client = pgcove_lib::supabase::SupabaseClient::new(&url, &key).unwrap();
    let info = client.project_info().await.unwrap();
    assert!(
        !info.rest_version.is_empty(),
        "no PostgREST version: {info:?}"
    );
    // Empty on a project with no buckets — not erroring is the point.
    client.list_buckets().await.unwrap();
}

#[test]
#[ignore = "not implemented — edge function listing needs a Supabase management access token (api.supabase.com), not the service-role key; fast-follow to pgcove issue #5"]
fn lists_edge_functions_via_management_api() {}

// Issue #11 (SSH tunnel connections) shipped: connect/auth/host-key-check
// and the local port-forward bridge are covered by src/ssh_tunnel.rs's unit
// tests and src/connections.rs's SSH config roundtrip + keyring tests.
// Mocking the SSH protocol isn't worth the scaffold's weight, so an
// end-to-end run needs a real bastion — set PGCOVE_TEST_SSH_TUNNEL and
// PGCOVE_TEST_SSH_TARGET_DB and run with `-- --ignored`:
//
// ```sh
// PGCOVE_TEST_SSH_TUNNEL=user:pass@bastion.example.com:22 \
// PGCOVE_TEST_SSH_TARGET_DB=127.0.0.1:5432 \
// cargo test -- --ignored connects_through_ssh_tunnel
// ```
#[tokio::test]
#[ignore = "requires a real SSH host reaching a Postgres server — set PGCOVE_TEST_SSH_TUNNEL=user:pass@host:port and PGCOVE_TEST_SSH_TARGET_DB=host:port"]
async fn connects_through_ssh_tunnel() {
    use pgcove_lib::connections::{DbKind, SshAuth, SshTunnelConfig};

    let spec = std::env::var("PGCOVE_TEST_SSH_TUNNEL")
        .expect("set PGCOVE_TEST_SSH_TUNNEL=user:pass@host:port");
    let (userpass, hostport) = spec.rsplit_once('@').expect("user:pass@host:port");
    let (user, pass) = userpass.split_once(':').expect("user:pass");
    let (host, port) = hostport.split_once(':').expect("host:port");
    let target = std::env::var("PGCOVE_TEST_SSH_TARGET_DB").expect(
        "set PGCOVE_TEST_SSH_TARGET_DB=host:port (Postgres address as reachable from the SSH host)",
    );
    let (target_host, target_port) = target.split_once(':').expect("host:port");

    let cfg = SshTunnelConfig {
        host: host.to_string(),
        port: port.parse().expect("SSH port"),
        user: user.to_string(),
        auth: SshAuth::Password,
    };
    let known_hosts_dir = tempfile::tempdir().expect("tempdir");
    let tunnel = pgcove_lib::ssh_tunnel::start(
        &cfg,
        pass,
        known_hosts_dir.path(),
        target_host.to_string(),
        target_port.parse().expect("target port"),
    )
    .await
    .expect("tunnel should start");

    let url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres?sslmode=prefer",
        tunnel.local_port
    );
    let db = pgcove_lib::db::connect(DbKind::Postgres, &url)
        .await
        .expect("connecting through the tunnel should succeed");
    let version = pgcove_lib::db::server_version(&db).await.unwrap();
    assert!(!version.is_empty(), "no server version: {version:?}");
}
