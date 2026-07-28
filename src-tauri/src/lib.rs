pub mod commands;
pub mod connections;
pub mod db;
pub mod known_hosts;
pub mod migrations;
pub mod settings;
pub mod ssh_tunnel;
pub mod supabase;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ssh_tunnel::SshTunnels::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_connections,
            commands::save_connection,
            commands::delete_connection,
            commands::test_connection,
            commands::list_tables,
            commands::table_rows,
            commands::table_structure,
            commands::primary_key_columns,
            commands::update_cell,
            commands::run_query,
            commands::list_policies,
            commands::create_policy_sql,
            commands::alter_policy_sql,
            commands::drop_policy_sql,
            commands::rls_sql,
            commands::execute_ddl,
            commands::list_auth_users,
            commands::migration_status,
            commands::apply_pending_migrations,
            commands::supabase_project_info,
            commands::supabase_list_buckets,
            commands::supabase_list_users,
            commands::supabase_ban_user,
            commands::supabase_delete_user,
            commands::supabase_list_edge_functions,
            commands::load_settings,
            commands::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
