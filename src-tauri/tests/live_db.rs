//! Integration tests against a real Postgres. There is no practical way to
//! mock the Postgres wire protocol the way dockshell mocks the Docker HTTP
//! API, so these are `#[ignore]`d and run locally/against a lab database:
//!
//! ```sh
//! PGCOVE_TEST_URL=postgres://user:pass@host:5432/db cargo test -- --ignored
//! ```

use pgcove_lib::connections::DbKind;
use pgcove_lib::db;
use pgcove_lib::db::RowsQuery;
use pgcove_lib::migrations;

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

async fn pool() -> db::Db {
    let url = std::env::var("PGCOVE_TEST_URL")
        .expect("set PGCOVE_TEST_URL=postgres://user:pass@host:5432/db");
    db::connect(DbKind::Postgres, &url)
        .await
        .expect("connect failed")
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
        let page = db::table_rows(&p, &t.schema, &t.name, &default_rows_query())
            .await
            .unwrap();
        assert!(page.rows.is_array());
    }
}

#[tokio::test]
#[ignore = "requires a reachable Postgres — set PGCOVE_TEST_URL"]
async fn pages_sorts_and_filters_table_rows() {
    let p = pool().await;
    let table = "pgcove_test_paging";

    db::execute_ddl(&p, &format!("DROP TABLE IF EXISTS {table}"))
        .await
        .unwrap();
    db::execute_ddl(&p, &format!("CREATE TABLE {table} (id int, name text)"))
        .await
        .unwrap();
    db::execute_ddl(
        &p,
        &format!(
            "INSERT INTO {table} (id, name) VALUES (1, 'banana'), (2, 'apple'), (3, 'cherry')"
        ),
    )
    .await
    .unwrap();
    // approx_total reads pg_class.reltuples, only refreshed by ANALYZE/VACUUM.
    db::execute_ddl(&p, &format!("ANALYZE {table}"))
        .await
        .unwrap();

    let sorted = db::table_rows(
        &p,
        "public",
        table,
        &RowsQuery {
            page: 1,
            page_size: 2,
            order_by: Some("name".into()),
            order_desc: false,
            filter_column: None,
            filter_value: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        sorted.rows,
        serde_json::json!([{"id": 2, "name": "apple"}, {"id": 1, "name": "banana"}])
    );
    assert_eq!(sorted.approx_total, 3);

    let page2 = db::table_rows(
        &p,
        "public",
        table,
        &RowsQuery {
            page: 2,
            page_size: 2,
            order_by: Some("name".into()),
            order_desc: false,
            filter_column: None,
            filter_value: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(page2.rows, serde_json::json!([{"id": 3, "name": "cherry"}]));

    let filtered = db::table_rows(
        &p,
        "public",
        table,
        &RowsQuery {
            page: 1,
            page_size: 10,
            order_by: None,
            order_desc: false,
            filter_column: Some("name".into()),
            filter_value: Some("an".into()),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        filtered.rows,
        serde_json::json!([{"id": 1, "name": "banana"}])
    );

    db::execute_ddl(&p, &format!("DROP TABLE {table}"))
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires a reachable Postgres — set PGCOVE_TEST_URL"]
async fn lists_rls_policies() {
    // Empty on a fresh database — asserting it doesn't error is the point.
    db::list_policies(&pool().await).await.unwrap();
}

#[tokio::test]
#[ignore = "requires a reachable Postgres — set PGCOVE_TEST_URL"]
async fn creates_and_drops_an_rls_policy() {
    let p = pool().await;
    let table = "pgcove_test_rls_roundtrip";

    db::execute_ddl(&p, &format!("DROP TABLE IF EXISTS {table}"))
        .await
        .unwrap();
    db::execute_ddl(&p, &format!("CREATE TABLE {table} (id int, user_id uuid)"))
        .await
        .unwrap();

    let enable_sql = db::rls_sql("public", table, true);
    assert!(enable_sql.contains("ENABLE ROW LEVEL SECURITY"));
    db::execute_ddl(&p, &enable_sql).await.unwrap();

    let draft = db::PolicyDraft {
        schema: "public".into(),
        table: table.into(),
        name: "own rows".into(),
        command: "SELECT".into(),
        roles: vec![],
        // A plain expression, not auth.uid() — this runs against local
        // Postgres too, which has no `auth` schema.
        using_expr: Some("user_id IS NOT NULL".into()),
        check_expr: None,
    };
    db::execute_ddl(&p, &db::create_policy_sql(&draft))
        .await
        .unwrap();

    let policies = db::list_policies(&p).await.unwrap();
    assert!(policies
        .iter()
        .any(|pol| pol.table == table && pol.name == "own rows"));

    db::execute_ddl(&p, &db::drop_policy_sql("public", table, "own rows"))
        .await
        .unwrap();
    let policies = db::list_policies(&p).await.unwrap();
    assert!(!policies.iter().any(|pol| pol.table == table));

    db::execute_ddl(&p, &format!("DROP TABLE {table}"))
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires a reachable Postgres — set PGCOVE_TEST_URL"]
async fn reads_table_structure() {
    let p = pool().await;
    let table = "pgcove_test_structure";

    db::execute_ddl(&p, &format!("DROP TABLE IF EXISTS {table}"))
        .await
        .unwrap();
    db::execute_ddl(
        &p,
        &format!(
            "CREATE TABLE {table} (
                id int PRIMARY KEY,
                owner_id int REFERENCES {table} (id),
                title text NOT NULL DEFAULT 'untitled'
            )"
        ),
    )
    .await
    .unwrap();
    db::execute_ddl(
        &p,
        &format!("CREATE UNIQUE INDEX {table}_title_idx ON {table} (title)"),
    )
    .await
    .unwrap();

    let structure = db::table_structure(&p, "public", table).await.unwrap();

    assert_eq!(structure.columns.len(), 3);
    let title = structure
        .columns
        .iter()
        .find(|c| c.name == "title")
        .unwrap();
    assert!(!title.nullable);
    assert_eq!(title.default.as_deref(), Some("'untitled'::text"));

    assert!(structure
        .indexes
        .iter()
        .any(|i| i.name == format!("{table}_title_idx") && i.is_unique));
    assert!(structure.indexes.iter().any(|i| i.is_primary));

    assert!(structure
        .constraints
        .iter()
        .any(|c| c.kind == "PRIMARY KEY" && c.columns == "id"));
    let fk = structure
        .constraints
        .iter()
        .find(|c| c.kind == "FOREIGN KEY")
        .unwrap();
    assert_eq!(fk.foreign_table.as_deref(), Some(table));

    db::execute_ddl(&p, &format!("DROP TABLE {table}"))
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires a reachable Postgres — set PGCOVE_TEST_URL"]
async fn runs_arbitrary_select_query() {
    let p = pool().await;
    let rows = db::run_query(&p, "select 1 as n, 'hi' as s").await.unwrap();
    assert_eq!(rows, serde_json::json!([{"n": 1, "s": "hi"}]));
}

#[tokio::test]
#[ignore = "requires a reachable Postgres — set PGCOVE_TEST_URL"]
async fn run_query_surfaces_postgres_errors() {
    let p = pool().await;
    let err = db::run_query(&p, "select * from no_such_table_xyz")
        .await
        .unwrap_err();
    assert!(err.contains("no_such_table_xyz"));
}

#[tokio::test]
#[ignore = "requires a reachable Postgres — set PGCOVE_TEST_URL"]
async fn run_query_rejects_duplicate_column_names() {
    // row_to_json emits duplicate JSON keys verbatim; decoding that into
    // serde_json::Value silently keeps only the last one. Catching this up
    // front beats quietly dropping a column's data — see db::run_query.
    let p = pool().await;
    let err = db::run_query(&p, "select 1 as id, 2 as id")
        .await
        .unwrap_err();
    assert!(err.contains("duplicate column"), "unexpected error: {err}");
}

#[tokio::test]
#[ignore = "requires a reachable Postgres — set PGCOVE_TEST_URL"]
async fn run_query_returns_empty_array_for_zero_rows() {
    // coalesce(json_agg(...), '[]'::json) is what keeps this an empty array
    // instead of SQL NULL — a naive json_agg-only wrapper would decode to
    // serde_json::Value::Null here instead, breaking a frontend that assumes
    // it always gets an array back.
    let p = pool().await;
    let rows = db::run_query(&p, "select 1 as n where false")
        .await
        .unwrap();
    assert_eq!(rows, serde_json::json!([]));
}

#[tokio::test]
#[ignore = "requires a reachable Postgres — set PGCOVE_TEST_URL"]
async fn applies_pending_migrations_and_records_them() {
    let p = pool().await;
    let table = "pgcove_test_schema_migrations";
    let migrated_table = "pgcove_test_migrated_table";

    db::execute_ddl(&p, &format!("DROP TABLE IF EXISTS {table}"))
        .await
        .unwrap();
    db::execute_ddl(&p, &format!("DROP TABLE IF EXISTS {migrated_table}"))
        .await
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("1_create_table.sql"),
        format!("CREATE TABLE {migrated_table} (id int);"),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("2_add_column.sql"),
        format!("ALTER TABLE {migrated_table} ADD COLUMN name text;"),
    )
    .unwrap();

    let before = migrations::migration_status(&p, dir.path(), Some(table))
        .await
        .unwrap();
    assert_eq!(before.len(), 2);
    assert!(before.iter().all(|m| !m.applied));

    let ran = migrations::apply_pending(&p, dir.path(), Some(table))
        .await
        .unwrap();
    assert_eq!(ran, vec!["1".to_string(), "2".to_string()]);

    let after = migrations::migration_status(&p, dir.path(), Some(table))
        .await
        .unwrap();
    assert!(after.iter().all(|m| m.applied));

    // Idempotent: re-running finds nothing pending left to apply.
    let ran_again = migrations::apply_pending(&p, dir.path(), Some(table))
        .await
        .unwrap();
    assert!(ran_again.is_empty());

    let structure = db::table_structure(&p, "public", migrated_table)
        .await
        .unwrap();
    assert_eq!(structure.columns.len(), 2);

    db::execute_ddl(&p, &format!("DROP TABLE {migrated_table}"))
        .await
        .unwrap();
    db::execute_ddl(&p, &format!("DROP TABLE {table}"))
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires a reachable Postgres — set PGCOVE_TEST_URL"]
async fn run_query_surfaces_syntax_errors_cleanly() {
    // A genuine parse error (as opposed to a valid-but-wrong-name error like
    // run_query_surfaces_postgres_errors above) must still come back as a
    // readable Err, not a panic — the wrapping subquery is exactly the kind
    // of place a naive implementation could turn a user typo into a Rust
    // panic instead of an on-screen message. Postgres error text is
    // locale-dependent (this dev box reports in Italian), so assert on the
    // quoted offending token it always includes verbatim rather than on
    // English wording like "syntax error".
    let p = pool().await;
    let err = db::run_query(&p, "selct 1").await.unwrap_err();
    assert!(
        !err.is_empty() && err.contains('"'),
        "expected a specific, readable parse error, got: {err}"
    );
}
