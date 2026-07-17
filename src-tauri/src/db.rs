//! Postgres access via sqlx. Everything the scaffold reads is cast to text /
//! JSON server-side so no client-side type mapping is needed — the query
//! editor (issue #1) and in-grid editing (issue #2) will need real typing.

use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

use crate::connections::ConnectionInfo;

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
/// plaintext for local dev servers without it.
pub fn connection_url(info: &ConnectionInfo, password: &str) -> String {
    format!(
        "postgres://{}:{}@{}:{}/{}?sslmode=prefer",
        encode_userinfo(&info.user),
        encode_userinfo(password),
        info.host,
        info.port,
        encode_userinfo(&info.database),
    )
}

pub async fn connect(url: &str) -> Result<PgPool, String> {
    // ponytail: one small pool per command invocation; cache pools per
    // connection id if latency ever matters.
    PgPoolOptions::new()
        .max_connections(2)
        .connect(url)
        .await
        .map_err(|e| e.to_string())
}

pub async fn server_version(pool: &PgPool) -> Result<String, String> {
    let row = sqlx::query("SELECT version()")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    row.try_get::<String, _>(0).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableInfo {
    pub schema: String,
    pub name: String,
    /// "BASE TABLE" | "VIEW" | ...
    pub kind: String,
}

pub async fn list_tables(pool: &PgPool) -> Result<Vec<TableInfo>, String> {
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

/// Double-quote a SQL identifier, escaping embedded quotes.
pub fn quote_ident(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// First 100 rows of a table as a JSON array of objects — the server does the
/// serialization (`row_to_json`), so arbitrary column types just work.
/// Pagination/sorting is issue #9.
pub async fn table_rows(
    pool: &PgPool,
    schema: &str,
    table: &str,
) -> Result<serde_json::Value, String> {
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

/// Run an arbitrary user query and return the result rows as JSON.
///
/// Wraps the statement as `SELECT row_to_json(t) FROM (<query>) t`, the same
/// trick `table_rows` uses — Postgres serializes every column type for us,
/// so no client-side type mapping is needed. The tradeoff: this only works
/// for a single SELECT-shaped statement. INSERT/UPDATE/DELETE/DDL need a
/// separate execute path returning rows-affected, which is a follow-up.
pub async fn run_query(pool: &PgPool, sql: &str) -> Result<serde_json::Value, String> {
    let trimmed = sql.trim().trim_end_matches(';').trim();
    if trimmed.is_empty() {
        return Err("empty query".to_string());
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

pub async fn list_policies(pool: &PgPool) -> Result<Vec<PolicyInfo>, String> {
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

pub async fn list_auth_users(pool: &PgPool) -> Result<Vec<AuthUser>, String> {
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

    fn info() -> ConnectionInfo {
        ConnectionInfo {
            id: "t".into(),
            name: "t".into(),
            host: "db.example.com".into(),
            port: 6543,
            user: "postgres.abc123".into(),
            database: "postgres".into(),
        }
    }

    #[test]
    fn connection_url_encodes_userinfo() {
        let url = connection_url(&info(), "p@ss:w/rd%");
        assert_eq!(
            url,
            "postgres://postgres.abc123:p%40ss%3Aw%2Frd%25@db.example.com:6543/postgres?sslmode=prefer"
        );
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
        assert_eq!(run_query(&pool, "   ;  ").await.unwrap_err(), "empty query");
    }
}
