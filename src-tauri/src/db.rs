//! Postgres access via sqlx. Everything the scaffold reads is cast to text /
//! JSON server-side so no client-side type mapping is needed — the query
//! editor (issue #1) and in-grid editing (issue #2) will need real typing.

use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Column, Executor, PgPool, Row, SqlitePool, Statement};

use crate::connections::{ConnectionInfo, DbKind};

/// An open connection pool, tagged by driver so callers can dispatch without
/// re-checking `ConnectionInfo::kind`.
pub enum Db {
    Postgres(PgPool),
    Sqlite(SqlitePool),
}

/// Percent-encode userinfo parts of a connection URL (user/password can
/// contain `@`, `:`, `/`, `%`...).
fn encode_userinfo(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `sslmode=prefer`: TLS when the server supports it (Supabase requires it),
/// plaintext for local dev servers without it. SQLite ignores host/port/user
/// entirely — `database` is the file path (or `:memory:`), and `?mode=rwc`
/// creates the file on first connect instead of erroring if it's missing.
pub fn connection_url(info: &ConnectionInfo, password: &str) -> String {
    match info.kind {
        DbKind::Postgres => format!(
            "postgres://{}:{}@{}:{}/{}?sslmode=prefer",
            encode_userinfo(&info.user),
            encode_userinfo(password),
            info.host,
            info.port,
            encode_userinfo(&info.database),
        ),
        DbKind::Sqlite => {
            if info.database == ":memory:" {
                "sqlite::memory:".to_string()
            } else {
                format!("sqlite://{}?mode=rwc", info.database)
            }
        }
    }
}

pub async fn connect(kind: DbKind, url: &str) -> Result<Db, String> {
    // ponytail: one small pool per command invocation; cache pools per
    // connection id if latency ever matters.
    match kind {
        DbKind::Postgres => PgPoolOptions::new()
            .max_connections(2)
            .connect(url)
            .await
            .map(Db::Postgres)
            .map_err(|e| e.to_string()),
        DbKind::Sqlite => {
            // `:memory:` gives each pooled connection its own separate
            // database — cap the pool at 1 so all queries share one.
            let max = if url.contains(":memory:") { 1 } else { 2 };
            SqlitePoolOptions::new()
                .max_connections(max)
                .connect(url)
                .await
                .map(Db::Sqlite)
                .map_err(|e| e.to_string())
        }
    }
}

pub async fn server_version(db: &Db) -> Result<String, String> {
    match db {
        Db::Postgres(pool) => {
            let row = sqlx::query("SELECT version()")
                .fetch_one(pool)
                .await
                .map_err(|e| e.to_string())?;
            row.try_get::<String, _>(0).map_err(|e| e.to_string())
        }
        Db::Sqlite(pool) => {
            let row = sqlx::query("SELECT sqlite_version()")
                .fetch_one(pool)
                .await
                .map_err(|e| e.to_string())?;
            row.try_get::<String, _>(0).map_err(|e| e.to_string())
        }
    }
}

/// Convert one SQLite row to a JSON object, keyed by column name. SQLite is
/// dynamically typed per-value, so decoding is tried integer, then float,
/// then text, then blob (hex-encoded), falling through to `null` — which is
/// also exactly what a real NULL does, since every `try_get` on it fails.
fn sqlite_row_to_json(row: &sqlx::sqlite::SqliteRow) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (i, col) in row.columns().iter().enumerate() {
        let value = if let Ok(v) = row.try_get::<i64, _>(i) {
            serde_json::Value::from(v)
        } else if let Ok(v) = row.try_get::<f64, _>(i) {
            serde_json::Value::from(v)
        } else if let Ok(v) = row.try_get::<String, _>(i) {
            serde_json::Value::from(v)
        } else if let Ok(v) = row.try_get::<Vec<u8>, _>(i) {
            serde_json::Value::from(v.iter().map(|b| format!("{b:02x}")).collect::<String>())
        } else {
            serde_json::Value::Null
        };
        map.insert(col.name().to_string(), value);
    }
    serde_json::Value::Object(map)
}

async fn sqlite_rows_to_json(pool: &SqlitePool, sql: &str) -> Result<serde_json::Value, String> {
    let rows = sqlx::query(sql)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::Value::Array(
        rows.iter().map(sqlite_row_to_json).collect(),
    ))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableInfo {
    pub schema: String,
    pub name: String,
    /// "BASE TABLE" | "VIEW" | ...
    pub kind: String,
}

pub async fn list_tables(db: &Db) -> Result<Vec<TableInfo>, String> {
    match db {
        Db::Postgres(pool) => {
            let rows = sqlx::query(
                "SELECT table_schema::text, table_name::text, table_type::text
                 FROM information_schema.tables
                 WHERE table_schema NOT IN ('pg_catalog', 'information_schema')
                 ORDER BY table_schema, table_name",
            )
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
            rows.iter()
                .map(|r| {
                    Ok(TableInfo {
                        schema: r.try_get(0).map_err(|e: sqlx::Error| e.to_string())?,
                        name: r.try_get(1).map_err(|e: sqlx::Error| e.to_string())?,
                        kind: r.try_get(2).map_err(|e: sqlx::Error| e.to_string())?,
                    })
                })
                .collect()
        }
        // SQLite has one implicit schema ("main") and its own catalog table
        // instead of information_schema.
        Db::Sqlite(pool) => {
            let rows = sqlx::query(
                "SELECT name, CASE type WHEN 'view' THEN 'VIEW' ELSE 'BASE TABLE' END
                 FROM sqlite_master
                 WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%'
                 ORDER BY name",
            )
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
            rows.iter()
                .map(|r| {
                    Ok(TableInfo {
                        schema: "main".to_string(),
                        name: r.try_get(0).map_err(|e: sqlx::Error| e.to_string())?,
                        kind: r.try_get(1).map_err(|e: sqlx::Error| e.to_string())?,
                    })
                })
                .collect()
        }
    }
}

/// Double-quote a SQL identifier, escaping embedded quotes.
pub fn quote_ident(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// First 100 rows of a table as a JSON array of objects. Pagination/sorting
/// is issue #9.
pub async fn table_rows(db: &Db, schema: &str, table: &str) -> Result<serde_json::Value, String> {
    match db {
        // The server does the serialization (`row_to_json`), so arbitrary
        // column types just work.
        Db::Postgres(pool) => {
            let sql = format!(
                "SELECT coalesce(json_agg(row_to_json(x)), '[]'::json)
                 FROM (SELECT * FROM {}.{} LIMIT 100) x",
                quote_ident(schema),
                quote_ident(table),
            );
            let row = sqlx::query(&sql)
                .fetch_one(pool)
                .await
                .map_err(|e| e.to_string())?;
            row.try_get::<serde_json::Value, _>(0)
                .map_err(|e| e.to_string())
        }
        // SQLite has one implicit schema — `schema` is ignored.
        Db::Sqlite(pool) => {
            let sql = format!("SELECT * FROM {} LIMIT 100", quote_ident(table));
            sqlite_rows_to_json(pool, &sql).await
        }
    }
}

/// Run an arbitrary user query and return the result rows as JSON.
///
/// Wraps the statement as `SELECT row_to_json(t) FROM (<query>) t`, the same
/// trick `table_rows` uses — Postgres serializes every column type for us,
/// so no client-side type mapping is needed. The tradeoff: this only works
/// for a single SELECT-shaped statement. INSERT/UPDATE/DELETE/DDL need a
/// separate execute path returning rows-affected, which is a follow-up.
pub async fn run_query(db: &Db, sql: &str) -> Result<serde_json::Value, String> {
    let trimmed = sql.trim().trim_end_matches(';').trim();
    if trimmed.is_empty() {
        return Err("empty query".to_string());
    }
    match db {
        Db::Postgres(pool) => {
            // row_to_json emits one JSON key per output column, including
            // duplicates verbatim (Postgres's `json` type doesn't dedupe).
            // When that text is decoded into a serde_json::Value on the way
            // back, duplicate keys silently collapse to the last one — a
            // self-join or a `SELECT *` across joined tables sharing a
            // column name (very common: `id`, `created_at`, ...) would
            // quietly drop data with no error. Describing the statement
            // first (parse-only, no execution — verified against a live
            // Postgres that neither DDL nor DML run any side effects here)
            // lets us catch that before running the wrapped query. If
            // describe fails for any other reason (bad SQL, non-SELECT
            // shape, ...), ignore it here and let the real error surface
            // from the actual execution below, unchanged.
            if let Ok(stmt) = pool.prepare(trimmed).await {
                let mut seen = std::collections::HashSet::new();
                let mut dupes: Vec<&str> = stmt
                    .columns()
                    .iter()
                    .map(|c| c.name())
                    .filter(|name| !seen.insert(*name))
                    .collect();
                dupes.dedup();
                if !dupes.is_empty() {
                    return Err(format!(
                        "query has duplicate column name(s) ({}); alias them to disambiguate, e.g. SELECT a.id AS a_id, b.id AS b_id",
                        dupes.join(", ")
                    ));
                }
            }
            let wrapped =
                format!("SELECT coalesce(json_agg(row_to_json(t)), '[]'::json) FROM ({trimmed}) t");
            let row = sqlx::query(&wrapped)
                .fetch_one(pool)
                .await
                .map_err(|e| e.to_string())?;
            row.try_get::<serde_json::Value, _>(0)
                .map_err(|e| e.to_string())
        }
        Db::Sqlite(pool) => {
            // sqlite_row_to_json builds a JSON object keyed by column name,
            // so a duplicate name would silently overwrite — same failure
            // mode as the Postgres path above, guarded the same way.
            if let Ok(stmt) = pool.prepare(trimmed).await {
                let mut seen = std::collections::HashSet::new();
                let mut dupes: Vec<&str> = stmt
                    .columns()
                    .iter()
                    .map(|c| c.name())
                    .filter(|name| !seen.insert(*name))
                    .collect();
                dupes.dedup();
                if !dupes.is_empty() {
                    return Err(format!(
                        "query has duplicate column name(s) ({}); alias them to disambiguate, e.g. SELECT a.id AS a_id, b.id AS b_id",
                        dupes.join(", ")
                    ));
                }
            }
            sqlite_rows_to_json(pool, trimmed).await
        }
    }
}

/// Row-level-security policies from the real `pg_policies` catalog — the
/// Supabase panel's data source. A proper RLS editor is issue #6.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyInfo {
    pub schema: String,
    pub table: String,
    pub name: String,
    pub command: String,
    pub roles: String,
    pub expression: String,
}

pub async fn list_policies(db: &Db) -> Result<Vec<PolicyInfo>, String> {
    let pool = match db {
        Db::Postgres(pool) => pool,
        Db::Sqlite(_) => {
            return Err("row-level security policies are a Postgres/Supabase feature".to_string())
        }
    };
    let rows = sqlx::query(
        "SELECT schemaname::text, tablename::text, policyname::text,
                coalesce(cmd::text, ''), coalesce(roles::text, ''), coalesce(qual::text, '')
         FROM pg_policies ORDER BY schemaname, tablename, policyname",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    rows.iter()
        .map(|r| {
            let g = |i: usize| r.try_get::<String, _>(i).map_err(|e| e.to_string());
            Ok(PolicyInfo {
                schema: g(0)?,
                table: g(1)?,
                name: g(2)?,
                command: g(3)?,
                roles: g(4)?,
                expression: g(5)?,
            })
        })
        .collect()
}

/// Supabase `auth.users`. Errors on a plain Postgres without that schema —
/// the frontend shows that as "not a Supabase database". Management-API
/// backed user admin is issue #7.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub created_at: String,
}

pub async fn list_auth_users(db: &Db) -> Result<Vec<AuthUser>, String> {
    let pool = match db {
        Db::Postgres(pool) => pool,
        Db::Sqlite(_) => {
            return Err("Supabase auth users are a Postgres/Supabase feature".to_string())
        }
    };
    let rows = sqlx::query(
        "SELECT id::text, coalesce(email::text, ''), coalesce(created_at::text, '')
         FROM auth.users ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    rows.iter()
        .map(|r| {
            let g = |i: usize| r.try_get::<String, _>(i).map_err(|e| e.to_string());
            Ok(AuthUser {
                id: g(0)?,
                email: g(1)?,
                created_at: g(2)?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(kind: DbKind) -> ConnectionInfo {
        ConnectionInfo {
            id: "t".into(),
            name: "t".into(),
            kind,
            host: "db.example.com".into(),
            port: 6543,
            user: "postgres.abc123".into(),
            database: "postgres".into(),
        }
    }

    #[test]
    fn connection_url_encodes_userinfo() {
        let url = connection_url(&info(DbKind::Postgres), "p@ss:w/rd%");
        assert_eq!(
            url,
            "postgres://postgres.abc123:p%40ss%3Aw%2Frd%25@db.example.com:6543/postgres?sslmode=prefer"
        );
    }

    #[test]
    fn connection_url_uses_sqlite_file_path() {
        let mut i = info(DbKind::Sqlite);
        i.database = "/tmp/some.db".into();
        assert_eq!(connection_url(&i, ""), "sqlite:///tmp/some.db?mode=rwc");
    }

    #[test]
    fn connection_url_uses_sqlite_memory_shorthand() {
        let mut i = info(DbKind::Sqlite);
        i.database = ":memory:".into();
        assert_eq!(connection_url(&i, ""), "sqlite::memory:");
    }

    #[test]
    fn quote_ident_escapes_quotes() {
        assert_eq!(quote_ident("users"), "\"users\"");
        assert_eq!(quote_ident("we\"ird"), "\"we\"\"ird\"");
    }

    #[tokio::test]
    async fn run_query_rejects_blank_input() {
        // connect_lazy doesn't touch the network, so this exercises the
        // empty-query guard without a real Postgres.
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://u:p@localhost/db")
            .unwrap();
        let db = Db::Postgres(pool);
        assert_eq!(run_query(&db, "   ;  ").await.unwrap_err(), "empty query");
    }

    // SQLite needs no external server, so — unlike the Postgres tests above,
    // which need a live database and are covered instead by the `#[ignore]`d
    // tests in tests/live_db.rs — these run for real in CI on every push.

    async fn sqlite_memory() -> Db {
        connect(DbKind::Sqlite, "sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn sqlite_reports_a_server_version() {
        let v = server_version(&sqlite_memory().await).await.unwrap();
        assert!(v.chars().next().unwrap().is_ascii_digit(), "got: {v}");
    }

    #[tokio::test]
    async fn sqlite_creates_inserts_and_lists_a_table() {
        let db = sqlite_memory().await;
        run_query(
            &db,
            "CREATE TABLE todos (id INTEGER PRIMARY KEY, title TEXT NOT NULL, done INTEGER)",
        )
        .await
        .unwrap();
        run_query(
            &db,
            "INSERT INTO todos (title, done) VALUES ('buy milk', 0)",
        )
        .await
        .unwrap();

        let tables = list_tables(&db).await.unwrap();
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].schema, "main");
        assert_eq!(tables[0].name, "todos");
        assert_eq!(tables[0].kind, "BASE TABLE");

        let rows = table_rows(&db, "main", "todos").await.unwrap();
        assert_eq!(
            rows,
            serde_json::json!([{"id": 1, "title": "buy milk", "done": 0}])
        );
    }

    #[tokio::test]
    async fn sqlite_run_query_returns_empty_array_for_zero_rows() {
        let rows = run_query(&sqlite_memory().await, "select 1 where 0")
            .await
            .unwrap();
        assert_eq!(rows, serde_json::json!([]));
    }

    #[tokio::test]
    async fn sqlite_run_query_rejects_duplicate_column_names() {
        let err = run_query(&sqlite_memory().await, "select 1 as id, 2 as id")
            .await
            .unwrap_err();
        assert!(err.contains("duplicate column"), "unexpected error: {err}");
    }

    #[tokio::test]
    async fn sqlite_policies_and_auth_users_are_postgres_only() {
        let db = sqlite_memory().await;
        assert!(list_policies(&db).await.unwrap_err().contains("Postgres"));
        assert!(list_auth_users(&db).await.unwrap_err().contains("Postgres"));
    }
}
