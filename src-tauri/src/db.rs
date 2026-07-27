//! Postgres access via sqlx. Everything the scaffold reads is cast to text /
//! JSON server-side so no client-side type mapping is needed — the query
//! editor (issue #1) and in-grid editing (issue #2) will need real typing.

use serde::{Deserialize, Serialize};
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

/// Page/sort/filter request for `table_rows` (issue #9). Column names go
/// through `quote_ident`; `filter_value` is always bound as a parameter,
/// never interpolated.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RowsQuery {
    pub page: u32,
    pub page_size: u32,
    pub order_by: Option<String>,
    pub order_desc: bool,
    pub filter_column: Option<String>,
    pub filter_value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowsPage {
    pub rows: serde_json::Value,
    /// `pg_class.reltuples` on Postgres — a fast estimate, not an exact
    /// count, and 0 for a freshly created table that hasn't been analyzed
    /// yet. SQLite uses a real `COUNT(*)`, fine at dev-db scale.
    pub approx_total: i64,
}

/// One page of a table's rows as a JSON array of objects, plus an
/// approximate total row count for the pager.
pub async fn table_rows(
    db: &Db,
    schema: &str,
    table: &str,
    query: &RowsQuery,
) -> Result<RowsPage, String> {
    let page = query.page.max(1);
    let page_size = query.page_size.max(1);
    let offset = (page - 1) * page_size;
    let order_clause = query
        .order_by
        .as_deref()
        .filter(|c| !c.trim().is_empty())
        .map(|c| {
            format!(
                " ORDER BY {} {}",
                quote_ident(c),
                if query.order_desc { "DESC" } else { "ASC" }
            )
        })
        .unwrap_or_default();
    let filter_col = query
        .filter_column
        .as_deref()
        .filter(|c| !c.trim().is_empty());

    match db {
        // The server does the serialization (`row_to_json`), so arbitrary
        // column types just work.
        Db::Postgres(pool) => {
            let where_clause = filter_col
                .map(|c| format!(" WHERE {}::text ILIKE '%' || $1 || '%'", quote_ident(c)))
                .unwrap_or_default();
            let sql = format!(
                "SELECT coalesce(json_agg(row_to_json(x)), '[]'::json)
                 FROM (SELECT * FROM {}.{}{where_clause}{order_clause} LIMIT {page_size} OFFSET {offset}) x",
                quote_ident(schema),
                quote_ident(table),
            );
            let mut q = sqlx::query(&sql);
            if !where_clause.is_empty() {
                q = q.bind(query.filter_value.clone().unwrap_or_default());
            }
            let row = q.fetch_one(pool).await.map_err(|e| e.to_string())?;
            let rows = row
                .try_get::<serde_json::Value, _>(0)
                .map_err(|e| e.to_string())?;

            let qualified = format!("{}.{}", quote_ident(schema), quote_ident(table));
            let approx_total = sqlx::query_scalar::<_, i64>(
                "SELECT coalesce(reltuples, 0)::bigint FROM pg_class WHERE oid = ($1)::regclass",
            )
            .bind(&qualified)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;

            Ok(RowsPage { rows, approx_total })
        }
        // SQLite has one implicit schema — `schema` is ignored.
        Db::Sqlite(pool) => {
            let where_clause = filter_col
                .map(|c| format!(" WHERE {} LIKE '%' || ? || '%'", quote_ident(c)))
                .unwrap_or_default();
            let sql = format!(
                "SELECT * FROM {}{where_clause}{order_clause} LIMIT {page_size} OFFSET {offset}",
                quote_ident(table),
            );
            let mut q = sqlx::query(&sql);
            if !where_clause.is_empty() {
                q = q.bind(query.filter_value.clone().unwrap_or_default());
            }
            let rows_raw = q.fetch_all(pool).await.map_err(|e| e.to_string())?;
            let rows = serde_json::Value::Array(rows_raw.iter().map(sqlite_row_to_json).collect());

            let count_sql = format!("SELECT COUNT(*) FROM {}{where_clause}", quote_ident(table));
            let mut cq = sqlx::query_scalar::<_, i64>(&count_sql);
            if !where_clause.is_empty() {
                cq = cq.bind(query.filter_value.clone().unwrap_or_default());
            }
            let approx_total = cq.fetch_one(pool).await.map_err(|e| e.to_string())?;

            Ok(RowsPage { rows, approx_total })
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

/// Form input for creating or altering an RLS policy. `using_expr` and
/// `check_expr` are user-authored SQL fragments and are passed through
/// verbatim — only identifiers (schema/table/policy/role names) go through
/// `quote_ident`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyDraft {
    pub schema: String,
    pub table: String,
    pub name: String,
    /// SELECT | INSERT | UPDATE | DELETE | ALL
    pub command: String,
    pub roles: Vec<String>,
    pub using_expr: Option<String>,
    pub check_expr: Option<String>,
}

fn roles_clause(roles: &[String]) -> String {
    if roles.is_empty() {
        return String::new();
    }
    format!(
        " TO {}",
        roles
            .iter()
            .map(|r| quote_ident(r))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn expr_clauses(using_expr: &Option<String>, check_expr: &Option<String>) -> String {
    let mut s = String::new();
    if let Some(u) = using_expr.as_deref().filter(|e| !e.trim().is_empty()) {
        s.push_str(&format!(" USING ({u})"));
    }
    if let Some(c) = check_expr.as_deref().filter(|e| !e.trim().is_empty()) {
        s.push_str(&format!(" WITH CHECK ({c})"));
    }
    s
}

/// `CREATE POLICY ...` for a new policy. Review before executing via
/// [`execute_ddl`].
pub fn create_policy_sql(p: &PolicyDraft) -> String {
    format!(
        "CREATE POLICY {} ON {}.{} FOR {}{}{};",
        quote_ident(&p.name),
        quote_ident(&p.schema),
        quote_ident(&p.table),
        p.command,
        roles_clause(&p.roles),
        expr_clauses(&p.using_expr, &p.check_expr),
    )
}

/// `ALTER POLICY ...` — roles/USING/CHECK only; Postgres doesn't allow
/// changing a policy's command (SELECT/INSERT/...) in place, so switching
/// command means drop + create instead.
pub fn alter_policy_sql(p: &PolicyDraft) -> String {
    format!(
        "ALTER POLICY {} ON {}.{}{}{};",
        quote_ident(&p.name),
        quote_ident(&p.schema),
        quote_ident(&p.table),
        roles_clause(&p.roles),
        expr_clauses(&p.using_expr, &p.check_expr),
    )
}

pub fn drop_policy_sql(schema: &str, table: &str, name: &str) -> String {
    format!(
        "DROP POLICY {} ON {}.{};",
        quote_ident(name),
        quote_ident(schema),
        quote_ident(table),
    )
}

/// `ALTER TABLE ... ENABLE/DISABLE ROW LEVEL SECURITY` — a policy without
/// RLS enabled on its table is a no-op, so this is exposed alongside policy
/// editing rather than buried in a separate table-structure view.
pub fn rls_sql(schema: &str, table: &str, enable: bool) -> String {
    format!(
        "ALTER TABLE {}.{} {} ROW LEVEL SECURITY;",
        quote_ident(schema),
        quote_ident(table),
        if enable { "ENABLE" } else { "DISABLE" },
    )
}

/// Execute a DDL/DML statement with no result rows expected — the confirmed
/// counterpart to the SQL the `*_sql` builders above generate for review.
pub async fn execute_ddl(db: &Db, sql: &str) -> Result<(), String> {
    match db {
        Db::Postgres(pool) => sqlx::query(sql)
            .execute(pool)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string()),
        Db::Sqlite(pool) => sqlx::query(sql)
            .execute(pool)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string()),
    }
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

/// A table's shape (issue #8) — columns, indexes, constraints. Postgres
/// catalog-only, like `list_policies`/`list_auth_users`; SQLite's structure
/// lives in a different catalog and is a separate follow-up if needed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexInfo {
    pub name: String,
    pub definition: String,
    pub is_unique: bool,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintInfo {
    pub name: String,
    /// PRIMARY KEY | FOREIGN KEY | UNIQUE | CHECK
    pub kind: String,
    pub columns: String,
    /// Target table for FOREIGN KEY constraints, e.g. "public.users".
    pub foreign_table: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableStructure {
    pub columns: Vec<ColumnInfo>,
    pub indexes: Vec<IndexInfo>,
    pub constraints: Vec<ConstraintInfo>,
}

pub async fn table_structure(db: &Db, schema: &str, table: &str) -> Result<TableStructure, String> {
    let pool = match db {
        Db::Postgres(pool) => pool,
        Db::Sqlite(_) => return Err("table structure is a Postgres/Supabase feature".to_string()),
    };

    let column_rows = sqlx::query(
        "SELECT column_name::text, data_type::text, (is_nullable = 'YES'), column_default::text
         FROM information_schema.columns
         WHERE table_schema = $1 AND table_name = $2
         ORDER BY ordinal_position",
    )
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let columns = column_rows
        .iter()
        .map(|r| {
            Ok(ColumnInfo {
                name: r.try_get(0).map_err(|e: sqlx::Error| e.to_string())?,
                data_type: r.try_get(1).map_err(|e: sqlx::Error| e.to_string())?,
                nullable: r.try_get(2).map_err(|e: sqlx::Error| e.to_string())?,
                default: r.try_get(3).map_err(|e: sqlx::Error| e.to_string())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let index_rows = sqlx::query(
        "SELECT i.relname::text, pg_get_indexdef(ix.indexrelid)::text, ix.indisunique, ix.indisprimary
         FROM pg_index ix
         JOIN pg_class t ON t.oid = ix.indrelid
         JOIN pg_class i ON i.oid = ix.indexrelid
         JOIN pg_namespace n ON n.oid = t.relnamespace
         WHERE n.nspname = $1 AND t.relname = $2
         ORDER BY i.relname",
    )
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let indexes = index_rows
        .iter()
        .map(|r| {
            Ok(IndexInfo {
                name: r.try_get(0).map_err(|e: sqlx::Error| e.to_string())?,
                definition: r.try_get(1).map_err(|e: sqlx::Error| e.to_string())?,
                is_unique: r.try_get(2).map_err(|e: sqlx::Error| e.to_string())?,
                is_primary: r.try_get(3).map_err(|e: sqlx::Error| e.to_string())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let constraint_rows = sqlx::query(
        "SELECT tc.constraint_name::text, tc.constraint_type::text,
                coalesce(string_agg(DISTINCT kcu.column_name, ', '), '')::text,
                (SELECT confrelid::regclass::text
                 FROM pg_constraint pc
                 JOIN pg_class c ON c.oid = pc.conrelid
                 JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE pc.conname = tc.constraint_name AND n.nspname = tc.table_schema
                   AND c.relname = tc.table_name AND pc.contype = 'f')
         FROM information_schema.table_constraints tc
         LEFT JOIN information_schema.key_column_usage kcu
           ON kcu.constraint_name = tc.constraint_name
          AND kcu.table_schema = tc.table_schema
          AND kcu.table_name = tc.table_name
         WHERE tc.table_schema = $1 AND tc.table_name = $2
         GROUP BY tc.constraint_name, tc.constraint_type, tc.table_schema, tc.table_name
         ORDER BY tc.constraint_name",
    )
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let constraints = constraint_rows
        .iter()
        .map(|r| {
            Ok(ConstraintInfo {
                name: r.try_get(0).map_err(|e: sqlx::Error| e.to_string())?,
                kind: r.try_get(1).map_err(|e: sqlx::Error| e.to_string())?,
                columns: r.try_get(2).map_err(|e: sqlx::Error| e.to_string())?,
                foreign_table: r.try_get(3).map_err(|e: sqlx::Error| e.to_string())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(TableStructure {
        columns,
        indexes,
        constraints,
    })
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
            supabase_url: None,
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

    fn draft() -> PolicyDraft {
        PolicyDraft {
            schema: "public".into(),
            table: "todos".into(),
            name: "own rows".into(),
            command: "SELECT".into(),
            roles: vec!["authenticated".into()],
            using_expr: Some("auth.uid() = user_id".into()),
            check_expr: None,
        }
    }

    #[test]
    fn create_policy_sql_builds_full_statement() {
        assert_eq!(
            create_policy_sql(&draft()),
            "CREATE POLICY \"own rows\" ON \"public\".\"todos\" FOR SELECT TO \"authenticated\" USING (auth.uid() = user_id);"
        );
    }

    #[test]
    fn create_policy_sql_omits_empty_roles_and_check() {
        let mut d = draft();
        d.roles.clear();
        assert_eq!(
            create_policy_sql(&d),
            "CREATE POLICY \"own rows\" ON \"public\".\"todos\" FOR SELECT USING (auth.uid() = user_id);"
        );
    }

    #[test]
    fn create_policy_sql_includes_with_check() {
        let mut d = draft();
        d.check_expr = Some("auth.uid() = user_id".into());
        assert!(create_policy_sql(&d).ends_with("WITH CHECK (auth.uid() = user_id);"));
    }

    #[test]
    fn alter_policy_sql_has_no_for_clause() {
        let sql = alter_policy_sql(&draft());
        assert_eq!(
            sql,
            "ALTER POLICY \"own rows\" ON \"public\".\"todos\" TO \"authenticated\" USING (auth.uid() = user_id);"
        );
        assert!(!sql.contains("FOR "));
    }

    #[test]
    fn drop_policy_sql_quotes_identifiers() {
        assert_eq!(
            drop_policy_sql("public", "todos", "own rows"),
            "DROP POLICY \"own rows\" ON \"public\".\"todos\";"
        );
    }

    #[test]
    fn rls_sql_toggles_enable_and_disable() {
        assert_eq!(
            rls_sql("public", "todos", true),
            "ALTER TABLE \"public\".\"todos\" ENABLE ROW LEVEL SECURITY;"
        );
        assert_eq!(
            rls_sql("public", "todos", false),
            "ALTER TABLE \"public\".\"todos\" DISABLE ROW LEVEL SECURITY;"
        );
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

        let page = table_rows(&db, "main", "todos", &default_rows_query())
            .await
            .unwrap();
        assert_eq!(
            page.rows,
            serde_json::json!([{"id": 1, "title": "buy milk", "done": 0}])
        );
        assert_eq!(page.approx_total, 1);
    }

    #[tokio::test]
    async fn sqlite_table_rows_pages_sorts_and_filters() {
        let db = sqlite_memory().await;
        run_query(
            &db,
            "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)",
        )
        .await
        .unwrap();
        for name in ["banana", "apple", "cherry"] {
            run_query(&db, &format!("INSERT INTO items (name) VALUES ('{name}')"))
                .await
                .unwrap();
        }

        let sorted = table_rows(
            &db,
            "main",
            "items",
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

        let page2 = table_rows(
            &db,
            "main",
            "items",
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

        let filtered = table_rows(
            &db,
            "main",
            "items",
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
        assert_eq!(filtered.approx_total, 1);
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

    #[tokio::test]
    async fn sqlite_table_structure_is_postgres_only() {
        let db = sqlite_memory().await;
        assert!(table_structure(&db, "main", "todos")
            .await
            .unwrap_err()
            .contains("Postgres"));
    }
}
