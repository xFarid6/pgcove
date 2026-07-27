//! Migrations view (issue #10): list applied migrations from the tracking
//! table and run pending `.sql` files from a local folder.
//!
//! Applied versions are read from `supabase_migrations.schema_migrations`
//! when present (same table + column shape the Supabase CLI uses — `version
//! text primary key, name text, statements text[]` — so the two tools stay
//! interoperable), falling back to a configurable table name (default
//! `public.schema_migrations`, created on first use) for non-Supabase
//! projects.

use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::{self, Db};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationInfo {
    pub version: String,
    pub name: String,
    pub applied: bool,
}

struct MigrationFile {
    version: String,
    name: String,
    path: PathBuf,
}

fn pg_pool(db: &Db) -> Result<&PgPool, String> {
    match db {
        Db::Postgres(pool) => Ok(pool),
        Db::Sqlite(_) => Err("migrations tracking is a Postgres/Supabase feature".to_string()),
    }
}

/// `<version>_<name>.sql` (the Supabase CLI convention, e.g.
/// `20240115120000_create_users.sql`) split on the first `_`; a file with no
/// underscore is treated as a bare version with an empty name. Sorted by
/// filename, which sorts correctly for the zero-padded timestamp convention.
fn list_local(folder: &Path) -> Result<Vec<MigrationFile>, String> {
    let mut paths: Vec<PathBuf> = fs::read_dir(folder)
        .map_err(|e| format!("{}: {e}", folder.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "sql"))
        .collect();
    paths.sort();
    Ok(paths
        .into_iter()
        .map(|path| {
            let stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let (version, name) = stem.split_once('_').unwrap_or((stem.as_str(), ""));
            MigrationFile {
                version: version.to_string(),
                name: name.to_string(),
                path,
            }
        })
        .collect())
}

/// `schema.table` or bare `table` (defaults to `public`), each part quoted.
fn qualify_table(raw: &str) -> String {
    match raw.splitn(2, '.').collect::<Vec<_>>().as_slice() {
        [schema, table] => format!("{}.{}", db::quote_ident(schema), db::quote_ident(table)),
        [table] => format!("public.{}", db::quote_ident(table)),
        _ => db::quote_ident(raw),
    }
}

async fn resolve_table(pool: &PgPool, table_override: Option<&str>) -> Result<String, String> {
    if let Some(t) = table_override.filter(|t| !t.trim().is_empty()) {
        return Ok(qualify_table(t.trim()));
    }
    let has_supabase_table: bool = sqlx::query_scalar(
        "SELECT to_regclass('supabase_migrations.schema_migrations') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(if has_supabase_table {
        "supabase_migrations.schema_migrations".to_string()
    } else {
        "public.schema_migrations".to_string()
    })
}

async fn applied_versions(pool: &PgPool, table: &str) -> Result<HashSet<String>, String> {
    sqlx::query_scalar::<_, String>(&format!("SELECT version FROM {table} ORDER BY version"))
        .fetch_all(pool)
        .await
        .map(|v| v.into_iter().collect())
        .map_err(|e| e.to_string())
}

/// No-op if the table already exists (real Supabase table or a fallback
/// table from a previous run).
async fn ensure_tracking_table(pool: &PgPool, table: &str) -> Result<(), String> {
    sqlx::query(&format!(
        "CREATE TABLE IF NOT EXISTS {table} (version text PRIMARY KEY, name text, statements text[])"
    ))
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub async fn migration_status(
    db: &Db,
    folder: &Path,
    table_override: Option<&str>,
) -> Result<Vec<MigrationInfo>, String> {
    let pool = pg_pool(db)?;
    let table = resolve_table(pool, table_override).await?;
    ensure_tracking_table(pool, &table).await?;
    let applied = applied_versions(pool, &table).await?;
    Ok(list_local(folder)?
        .into_iter()
        .map(|f| MigrationInfo {
            applied: applied.contains(&f.version),
            version: f.version,
            name: f.name,
        })
        .collect())
}

/// Single-quoted SQL string literal, embedded doubling to escape `'`. Used
/// only for the tracking-table insert appended below — every other value
/// that reaches SQL text in this module is a schema/table identifier via
/// `quote_ident`/`qualify_table`.
fn sql_literal(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Runs pending `.sql` files in filename order, each file's content plus its
/// tracking-table insert wrapped in one `BEGIN ... COMMIT` and sent as a
/// single multi-statement string via [`sqlx::raw_sql`] — the same execute-on-pool
/// shape `db::execute_ddl` already uses, which sidesteps a known sqlx/rustc
/// HRTB inference failure ("implementation of Executor is not general
/// enough") that a `pool.begin()`-based `Transaction` handle triggers when
/// called from inside a `#[tauri::command]` async fn. Stops at the first
/// failure and returns the versions applied before that point.
pub async fn apply_pending(
    db: &Db,
    folder: &Path,
    table_override: Option<&str>,
) -> Result<Vec<String>, String> {
    let pool = pg_pool(db)?;
    let table = resolve_table(pool, table_override).await?;
    ensure_tracking_table(pool, &table).await?;
    let applied = applied_versions(pool, &table).await?;

    let mut ran = Vec::new();
    for f in list_local(folder)?
        .into_iter()
        .filter(|f| !applied.contains(&f.version))
    {
        let sql = fs::read_to_string(&f.path).map_err(|e| format!("{}: {e}", f.path.display()))?;
        let combined = format!(
            "BEGIN;\n{sql}\nINSERT INTO {table} (version, name) VALUES ({}, {});\nCOMMIT;",
            sql_literal(&f.version),
            sql_literal(&f.name),
        );
        sqlx::raw_sql(&combined)
            .execute(pool)
            .await
            .map_err(|e| format!("{} failed: {e}", f.path.display()))?;
        ran.push(f.version);
    }
    Ok(ran)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_local_parses_version_and_name_and_sorts() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("2_second.sql"), "").unwrap();
        fs::write(dir.path().join("1_first.sql"), "").unwrap();
        fs::write(dir.path().join("notes.txt"), "").unwrap();
        fs::write(dir.path().join("bareversion.sql"), "").unwrap();

        let files = list_local(dir.path()).unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].version, "1");
        assert_eq!(files[0].name, "first");
        assert_eq!(files[1].version, "2");
        assert_eq!(files[1].name, "second");
        assert_eq!(files[2].version, "bareversion");
        assert_eq!(files[2].name, "");
    }

    #[test]
    fn qualify_table_defaults_to_public_schema() {
        assert_eq!(
            qualify_table("schema_migrations"),
            "public.\"schema_migrations\""
        );
        assert_eq!(
            qualify_table("supabase_migrations.schema_migrations"),
            "\"supabase_migrations\".\"schema_migrations\""
        );
    }

    #[tokio::test]
    async fn sqlite_migrations_are_postgres_only() {
        let db = db::connect(crate::connections::DbKind::Sqlite, "sqlite::memory:")
            .await
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        assert!(migration_status(&db, dir.path(), None)
            .await
            .unwrap_err()
            .contains("Postgres"));
    }
}
