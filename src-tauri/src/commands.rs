//! Tauri commands — the IPC surface the Vue frontend calls via `invoke`.

use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;

use crate::connections::{self, ConnectionInfo, DbKind};
use crate::db::{
    self, AuthUser, Db, PolicyDraft, PolicyInfo, RowsPage, RowsQuery, TableInfo, TableStructure,
};
use crate::migrations::{self, MigrationInfo};
use crate::ssh_tunnel::{self, SshTunnels, TunnelHandle};
use crate::supabase::{AdminUser, ProjectInfo, StorageBucket, SupabaseClient};

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
/// saved connection, service-role key from the keyring.
fn supabase_for(app: &tauri::AppHandle, id: &str) -> Result<SupabaseClient, String> {
    let info = connections::get(&store_dir(app)?, id)?;
    let url = info
        .supabase_url
        .filter(|u| !u.trim().is_empty())
        .ok_or_else(|| "this connection is not a Supabase project".to_string())?;
    let key = connections::get_service_key(id)
        .map_err(|e| format!("no service-role key saved for this connection ({e})"))?;
    SupabaseClient::new(&url, &key)
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
) -> Result<(), String> {
    // Evict any tunnel already running under the old config — otherwise an
    // edited bastion/auth method wouldn't take effect until app restart.
    app.state::<SshTunnels>().0.lock().unwrap().remove(&info.id);
    connections::save(&store_dir(&app)?, info, password, service_key, ssh_secret)
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
    db::run_query(&pool, &sql).await
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
