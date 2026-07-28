//! Tauri commands — the IPC surface the Vue frontend calls via `invoke`.

use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;

use crate::connections::{self, ConnectionInfo, DbKind};
use crate::db::{
    self, AuthUser, Db, ErdData, PolicyDraft, PolicyInfo, RowsPage, RowsQuery, TableInfo,
    TableStructure,
};
use crate::migrations::{self, MigrationInfo};
use crate::queries_history::{self, QueryRecord};
use crate::settings::{self};
use crate::ssh_tunnel::{self, SshTunnels, TunnelHandle};
use crate::supabase::{AdminUser, EdgeFunction, ProjectInfo, StorageBucket, SupabaseClient};

fn store_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}

/// Returns the active tunnel for `id`, starting one if this is the first
/// call since the app launched (or since the connection was last saved/
/// deleted — see `save_connection`/`delete_connection`, which evict the
/// cached entry so an edited or removed tunnel config can't linger).
async fn ensure_tunnel(
    app: &tauri::AppHandle,
    id: &str,
    info: &ConnectionInfo,
) -> Result<Arc<TunnelHandle>, String> {
    let cfg = info
        .ssh_tunnel
        .as_ref()
        .ok_or_else(|| "this connection has no SSH tunnel configured".to_string())?;
    if let Some(existing) = app.state::<SshTunnels>().0.lock().unwrap().get(id) {
        return Ok(existing.clone());
    }
    // Missing keyring entry just means "no secret" — fine for a passphrase-
    // less key; password auth without one fails downstream with a clear
    // "could not sign in" message instead of a keyring-specific error here.
    let secret = connections::get_ssh_secret(id).unwrap_or_default();
    let handle = Arc::new(
        ssh_tunnel::start(cfg, &secret, &store_dir(app)?, info.host.clone(), info.port).await?,
    );
    app.state::<SshTunnels>()
        .0
        .lock()
        .unwrap()
        .insert(id.to_string(), handle.clone());
    Ok(handle)
}

async fn pool_for(app: &tauri::AppHandle, id: &str) -> Result<Db, String> {
    let info = connections::get(&store_dir(app)?, id)?;
    // SQLite connections have no password — nothing is ever written to the
    // keyring for them, so looking one up would just error.
    let password = match info.kind {
        DbKind::Sqlite => String::new(),
        DbKind::Postgres | DbKind::MySql => connections::get_password(id)?,
    };
    let mut effective = info.clone();
    if info.ssh_tunnel.is_some() {
        let tunnel = ensure_tunnel(app, id, &info).await?;
        effective.host = "127.0.0.1".to_string();
        effective.port = tunnel.local_port;
    }
    db::connect(effective.kind, &db::connection_url(&effective, &password)).await
}

/// Same shape as `pool_for`, for the Supabase HTTP APIs: project URL from the
/// saved connection, service-role key and optional management token from the keyring.
fn supabase_for(app: &tauri::AppHandle, id: &str) -> Result<SupabaseClient, String> {
    let info = connections::get(&store_dir(app)?, id)?;
    let url = info
        .supabase_url
        .filter(|u| !u.trim().is_empty())
        .ok_or_else(|| "this connection is not a Supabase project".to_string())?;
    let key = connections::get_service_key(id)
        .map_err(|e| format!("no service-role key saved for this connection ({e})"))?;
    let mgmt_token = connections::get_mgmt_token(id).ok();
    SupabaseClient::new_with_mgmt_token(&url, &key, mgmt_token)
}

#[tauri::command]
pub fn list_connections(app: tauri::AppHandle) -> Result<Vec<ConnectionInfo>, String> {
    connections::load(&store_dir(&app)?)
}

#[tauri::command]
pub fn save_connection(
    app: tauri::AppHandle,
    info: ConnectionInfo,
    password: Option<String>,
    service_key: Option<String>,
    ssh_secret: Option<String>,
    mgmt_token: Option<String>,
) -> Result<(), String> {
    // Evict any tunnel already running under the old config — otherwise an
    // edited bastion/auth method wouldn't take effect until app restart.
    app.state::<SshTunnels>().0.lock().unwrap().remove(&info.id);
    connections::save(
        &store_dir(&app)?,
        info,
        password,
        service_key,
        ssh_secret,
        mgmt_token,
    )
}

#[tauri::command]
pub fn delete_connection(app: tauri::AppHandle, id: String) -> Result<(), String> {
    app.state::<SshTunnels>().0.lock().unwrap().remove(&id);
    connections::delete(&store_dir(&app)?, &id)
}

/// Connect and report `SELECT version()` — the "test connection" button.
#[tauri::command]
pub async fn test_connection(app: tauri::AppHandle, id: String) -> Result<String, String> {
    let pool = pool_for(&app, &id).await?;
    db::server_version(&pool).await
}

/// Health check (ping) — on-demand connection reachability test (issue #33).
#[tauri::command]
pub async fn ping_connection(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let pool = pool_for(&app, &id).await?;
    db::ping(&pool).await
}

#[tauri::command]
pub async fn list_tables(
    app: tauri::AppHandle,
    connection_id: String,
) -> Result<Vec<TableInfo>, String> {
    let pool = pool_for(&app, &connection_id).await?;
    db::list_tables(&pool).await
}

#[tauri::command]
pub async fn table_rows(
    app: tauri::AppHandle,
    connection_id: String,
    schema: String,
    table: String,
    query: RowsQuery,
) -> Result<RowsPage, String> {
    let pool = pool_for(&app, &connection_id).await?;
    db::table_rows(&pool, &schema, &table, &query).await
}

#[tauri::command]
pub async fn run_query(
    app: tauri::AppHandle,
    connection_id: String,
    sql: String,
) -> Result<serde_json::Value, String> {
    let pool = pool_for(&app, &connection_id).await?;
    let result = db::run_query(&pool, &sql).await?;

    // Add to history after successful execution.
    let _ = queries_history::add_query(&store_dir(&app)?, sql, connection_id);

    Ok(result)
}

#[tauri::command]
pub async fn table_structure(
    app: tauri::AppHandle,
    connection_id: String,
    schema: String,
    table: String,
) -> Result<TableStructure, String> {
    let pool = pool_for(&app, &connection_id).await?;
    db::table_structure(&pool, &schema, &table).await
}

#[tauri::command]
pub async fn erd_data(
    app: tauri::AppHandle,
    connection_id: String,
    schema: String,
) -> Result<ErdData, String> {
    let pool = pool_for(&app, &connection_id).await?;
    db::erd_data(&pool, &schema).await
}

#[tauri::command]
pub async fn primary_key_columns(
    app: tauri::AppHandle,
    connection_id: String,
    schema: String,
    table: String,
) -> Result<Vec<String>, String> {
    let pool = pool_for(&app, &connection_id).await?;
    db::primary_key_columns(&pool, &schema, &table).await
}

#[tauri::command]
pub async fn update_cell(
    app: tauri::AppHandle,
    connection_id: String,
    schema: String,
    table: String,
    pk: std::collections::HashMap<String, Option<String>>,
    column: String,
    value: Option<String>,
) -> Result<(), String> {
    let pool = pool_for(&app, &connection_id).await?;
    db::update_cell(&pool, &schema, &table, &pk, &column, value.as_deref()).await
}

#[tauri::command]
pub async fn list_policies(
    app: tauri::AppHandle,
    connection_id: String,
) -> Result<Vec<PolicyInfo>, String> {
    let pool = pool_for(&app, &connection_id).await?;
    db::list_policies(&pool).await
}

#[tauri::command]
pub async fn list_auth_users(
    app: tauri::AppHandle,
    connection_id: String,
) -> Result<Vec<AuthUser>, String> {
    let pool = pool_for(&app, &connection_id).await?;
    db::list_auth_users(&pool).await
}

#[tauri::command]
pub async fn migration_status(
    app: tauri::AppHandle,
    connection_id: String,
    folder: String,
    table: Option<String>,
) -> Result<Vec<MigrationInfo>, String> {
    let pool = pool_for(&app, &connection_id).await?;
    migrations::migration_status(&pool, std::path::Path::new(&folder), table.as_deref()).await
}

/// Runs pending `.sql` files in `folder`; see `migrations::apply_pending`.
#[tauri::command]
pub async fn apply_pending_migrations(
    app: tauri::AppHandle,
    connection_id: String,
    folder: String,
    table: Option<String>,
) -> Result<Vec<String>, String> {
    let pool = pool_for(&app, &connection_id).await?;
    migrations::apply_pending(&pool, std::path::Path::new(&folder), table.as_deref()).await
}

#[tauri::command]
pub fn create_policy_sql(draft: PolicyDraft) -> String {
    db::create_policy_sql(&draft)
}

#[tauri::command]
pub fn alter_policy_sql(draft: PolicyDraft) -> String {
    db::alter_policy_sql(&draft)
}

#[tauri::command]
pub fn drop_policy_sql(schema: String, table: String, name: String) -> String {
    db::drop_policy_sql(&schema, &table, &name)
}

#[tauri::command]
pub fn rls_sql(schema: String, table: String, enable: bool) -> String {
    db::rls_sql(&schema, &table, enable)
}

/// Runs a statement the user already confirmed via one of the `*_sql`
/// preview commands above.
#[tauri::command]
pub async fn execute_ddl(
    app: tauri::AppHandle,
    connection_id: String,
    sql: String,
) -> Result<(), String> {
    let pool = pool_for(&app, &connection_id).await?;
    db::execute_ddl(&pool, &sql).await
}

/// Import rows into a table from a CSV or JSON file (issue #32).
#[tauri::command]
pub async fn import_rows_from_file(
    app: tauri::AppHandle,
    connection_id: String,
    schema: String,
    table: String,
    file_path: String,
) -> Result<(), String> {
    let content = tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| format!("failed to read file: {}", e))?;

    let rows = if file_path.ends_with(".csv") {
        parse_csv(&content)?
    } else if file_path.ends_with(".json") {
        parse_json(&content)?
    } else {
        return Err("unsupported file type — use .csv or .json".to_string());
    };

    if rows.is_empty() {
        return Err("no rows to import".to_string());
    }

    let pool = pool_for(&app, &connection_id).await?;
    db::insert_rows_batch(&pool, &schema, &table, rows).await
}

/// Parse CSV content into rows.
fn parse_csv(content: &str) -> Result<Vec<serde_json::Value>, String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Ok(vec![]);
    }

    let headers = parse_csv_line(lines[0])?;
    let mut rows = vec![];

    for line in &lines[1..] {
        if line.trim().is_empty() {
            continue;
        }
        let values = parse_csv_line(line)?;
        let mut row = serde_json::json!({});
        for (i, header) in headers.iter().enumerate() {
            row[header] = serde_json::json!(values.get(i).cloned().unwrap_or_default());
        }
        rows.push(row);
    }

    Ok(rows)
}

/// Parse a CSV line, handling quoted fields.
fn parse_csv_line(line: &str) -> Result<Vec<String>, String> {
    let mut fields = vec![];
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            '"' => in_quotes = true,
            ',' if !in_quotes => {
                fields.push(current);
                current = String::new();
            }
            _ => current.push(c),
        }
    }
    fields.push(current);
    Ok(fields)
}

/// Parse JSON content into rows.
fn parse_json(content: &str) -> Result<Vec<serde_json::Value>, String> {
    let data: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("invalid JSON: {}", e))?;

    let arr = data
        .as_array()
        .ok_or_else(|| "JSON must be an array of objects".to_string())?;

    Ok(arr.clone())
}

/// Import rows into a table from parsed data (used when content is pre-parsed).
#[tauri::command]
pub async fn import_rows(
    app: tauri::AppHandle,
    connection_id: String,
    schema: String,
    table: String,
    rows: Vec<serde_json::Value>,
) -> Result<(), String> {
    let pool = pool_for(&app, &connection_id).await?;
    db::insert_rows_batch(&pool, &schema, &table, rows).await
}

/// Project self-check over HTTP — reachability plus the PostgREST version,
/// which is the most a service-role key can tell us about the project itself.
#[tauri::command]
pub async fn supabase_project_info(
    app: tauri::AppHandle,
    connection_id: String,
) -> Result<ProjectInfo, String> {
    supabase_for(&app, &connection_id)?.project_info().await
}

#[tauri::command]
pub async fn supabase_list_buckets(
    app: tauri::AppHandle,
    connection_id: String,
) -> Result<Vec<StorageBucket>, String> {
    supabase_for(&app, &connection_id)?.list_buckets().await
}

/// Admin user listing over the Management/data-plane auth API (issue #7).
#[tauri::command]
pub async fn supabase_list_users(
    app: tauri::AppHandle,
    connection_id: String,
    page: u32,
    per_page: u32,
) -> Result<Vec<AdminUser>, String> {
    supabase_for(&app, &connection_id)?
        .list_users(page, per_page)
        .await
}

/// `ban_duration` is a GoTrue duration string, e.g. `"24h"`; pass `"none"` to unban.
#[tauri::command]
pub async fn supabase_ban_user(
    app: tauri::AppHandle,
    connection_id: String,
    user_id: String,
    ban_duration: String,
) -> Result<(), String> {
    supabase_for(&app, &connection_id)?
        .ban_user(&user_id, &ban_duration)
        .await
}

#[tauri::command]
pub async fn supabase_delete_user(
    app: tauri::AppHandle,
    connection_id: String,
    user_id: String,
) -> Result<(), String> {
    supabase_for(&app, &connection_id)?
        .delete_user(&user_id)
        .await
}

/// Edge functions via the Management API (issue #30). Requires a personal
/// management access token stored separately from the service-role key.
#[tauri::command]
pub async fn supabase_list_edge_functions(
    app: tauri::AppHandle,
    connection_id: String,
) -> Result<Vec<EdgeFunction>, String> {
    supabase_for(&app, &connection_id)?
        .list_edge_functions()
        .await
}

#[tauri::command]
pub fn load_settings(app: tauri::AppHandle) -> Result<crate::settings::AppSettings, String> {
    settings::load(&store_dir(&app)?)
}

#[tauri::command]
pub fn save_settings(
    app: tauri::AppHandle,
    settings: crate::settings::AppSettings,
) -> Result<(), String> {
    settings::save(&store_dir(&app)?, &settings)
}

#[tauri::command]
pub fn add_query_to_history(
    app: tauri::AppHandle,
    connection_id: String,
    sql: String,
) -> Result<(), String> {
    queries_history::add_query(&store_dir(&app)?, sql, connection_id)
}

#[tauri::command]
pub fn list_query_history(app: tauri::AppHandle) -> Result<Vec<QueryRecord>, String> {
    queries_history::list_queries(&store_dir(&app)?)
}

#[tauri::command]
pub fn delete_query_from_history(app: tauri::AppHandle, id: String) -> Result<(), String> {
    queries_history::delete_query(&store_dir(&app)?, &id)
}

#[tauri::command]
pub fn clear_query_history(app: tauri::AppHandle) -> Result<(), String> {
    queries_history::clear_queries(&store_dir(&app)?)
}
