pub mod cli;
pub mod commands;
pub mod contracts;
pub mod data;
pub mod shell;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = commands::AppState::new();

    tauri::Builder::default()
        .plugin(shell::autostart::build_plugin())
        .plugin(shell::shortcut::build_plugin())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_tray_snapshot,
            commands::get_dashboard_snapshot,
            commands::get_sessions_view,
            commands::get_session_detail,
            commands::trigger_sync,
            commands::open_dashboard,
            commands::open_settings,
            commands::take_pending_settings_section,
            commands::dismiss_tray,
            commands::resize_tray,
            commands::quit_app,
            commands::get_preferences,
            commands::update_preferences,
            commands::get_cli_info,
            commands::install_or_upgrade_cli,
            commands::export_json,
            commands::get_sources_health,
            commands::get_tokscale_settings,
            commands::update_tokscale_settings,
            commands::recover_tokscale_timezone,
            commands::get_custom_pricing,
            commands::save_custom_pricing,
            commands::lookup_model_pricing,
            commands::refresh_pricing_catalog,
            commands::get_providers_status,
            commands::run_provider_action,
            commands::get_social_status,
            commands::run_social_action,
            commands::configure_autosubmit,
            commands::reveal_config_path,
            commands::pick_path,
        ])
        .setup(|app| {
            if app
                .config()
                .plugins
                .0
                .get("updater")
                .is_some_and(|v| v.is_object())
            {
                let _ = app
                    .handle()
                    .plugin(tauri_plugin_updater::Builder::new().build());
            }
            commands::setup_app(app)
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
