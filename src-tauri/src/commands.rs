//! Tauri commands — the IPC surface the Vue frontend calls via `invoke`.

use std::path::PathBuf;
use tauri::Manager;

use crate::connections::{self, ConnectionInfo};
use crate::db::{self, AuthUser, PolicyInfo, TableInfo};

fn store_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}

async fn pool_for(app: &tauri::AppHandle, id: &str) -> Result<sqlx::PgPool, String> {
    let info = connections::get(&store_dir(app)?, id)?;
    let password = connections::get_password(id)?;
    db::connect(&db::connection_url(&info, &password)).await
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
) -> Result<(), String> {
    connections::save(&store_dir(&app)?, info, password)
}

#[tauri::command]
pub fn delete_connection(app: tauri::AppHandle, id: String) -> Result<(), String> {
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
) -> Result<serde_json::Value, String> {
    let pool = pool_for(&app, &connection_id).await?;
    db::table_rows(&pool, &schema, &table).await
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
