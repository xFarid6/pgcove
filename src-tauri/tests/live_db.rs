//! Integration tests against a real Postgres. There is no practical way to
//! mock the Postgres wire protocol the way dockshell mocks the Docker HTTP
//! API, so these are `#[ignore]`d and run locally/against a lab database:
//!
//! ```sh
//! PGCOVE_TEST_URL=postgres://user:pass@host:5432/db cargo test -- --ignored
//! ```

use pgcove_lib::db;

async fn pool() -> sqlx::PgPool {
    let url = std::env::var("PGCOVE_TEST_URL")
        .expect("set PGCOVE_TEST_URL=postgres://user:pass@host:5432/db");
    db::connect(&url).await.expect("connect failed")
}

#[tokio::test]
#[ignore = "requires a reachable Postgres — set PGCOVE_TEST_URL"]
async fn reports_server_version() {
    let v = db::server_version(&pool().await).await.unwrap();
    assert!(v.contains("PostgreSQL"));
}

#[tokio::test]
#[ignore = "requires a reachable Postgres — set PGCOVE_TEST_URL"]
async fn lists_tables_and_reads_rows() {
    let p = pool().await;
    let tables = db::list_tables(&p).await.unwrap();
    if let Some(t) = tables.first() {
        let rows = db::table_rows(&p, &t.schema, &t.name).await.unwrap();
        assert!(rows.is_array());
    }
}

#[tokio::test]
#[ignore = "requires a reachable Postgres — set PGCOVE_TEST_URL"]
async fn lists_rls_policies() {
    // Empty on a fresh database — asserting it doesn't error is the point.
    db::list_policies(&pool().await).await.unwrap();
}
