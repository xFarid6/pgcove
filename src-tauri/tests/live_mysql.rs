//! Integration tests against a real MySQL/MariaDB server. Mirrors
//! `live_db.rs`'s pattern for Postgres — `#[ignore]`d, run locally/against a
//! lab database:
//!
//! ```sh
//! MYSQL_TEST_URL=mysql://user:pass@host:3306/db cargo test -- --ignored
//! ```

use pgcove_lib::connections::DbKind;
use pgcove_lib::db::{self, Db, RowsQuery};

fn default_rows_query() -> RowsQuery {
    RowsQuery {
        page: 1,
        page_size: 100,
        order_by: None,
        order_desc: false,
        filter_column: None,
        filter_value: None,
    }
}

async fn pool() -> Db {
    let url =
        std::env::var("MYSQL_TEST_URL").expect("set MYSQL_TEST_URL=mysql://user:pass@host:3306/db");
    db::connect(DbKind::MySql, &url)
        .await
        .expect("connect failed")
}

#[tokio::test]
#[ignore = "requires a reachable MySQL — set MYSQL_TEST_URL"]
async fn reports_server_version() {
    let v = db::server_version(&pool().await).await.unwrap();
    assert!(!v.is_empty());
}

#[tokio::test]
#[ignore = "requires a reachable MySQL — set MYSQL_TEST_URL"]
async fn lists_tables_and_reads_rows() {
    let p = pool().await;
    let tables = db::list_tables(&p).await.unwrap();
    if let Some(t) = tables.first() {
        let page = db::table_rows(&p, &t.schema, &t.name, &default_rows_query())
            .await
            .unwrap();
        assert!(page.rows.is_array());
    }
}

#[tokio::test]
#[ignore = "requires a reachable MySQL — set MYSQL_TEST_URL"]
async fn query_editor_is_not_yet_supported() {
    let err = db::run_query(&pool().await, "select 1").await.unwrap_err();
    assert!(err.contains("MySQL"));
}

#[tokio::test]
#[ignore = "requires a reachable MySQL — set MYSQL_TEST_URL"]
async fn rls_and_auth_users_are_postgres_only() {
    let p = pool().await;
    assert!(db::list_policies(&p).await.is_err());
    assert!(db::list_auth_users(&p).await.is_err());
}
