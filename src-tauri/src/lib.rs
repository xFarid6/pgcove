pub mod commands;
pub mod connections;
pub mod db;
pub mod supabase;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_connections,
            commands::save_connection,
            commands::delete_connection,
            commands::test_connection,
            commands::list_tables,
            commands::table_rows,
            commands::table_structure,
            commands::run_query,
            commands::list_policies,
            commands::create_policy_sql,
            commands::alter_policy_sql,
            commands::drop_policy_sql,
            commands::rls_sql,
            commands::execute_ddl,
            commands::list_auth_users,
            commands::supabase_project_info,
            commands::supabase_list_buckets,
            commands::supabase_list_users,
            commands::supabase_ban_user,
            commands::supabase_delete_user,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
