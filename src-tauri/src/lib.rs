mod commands;
mod system_theme;
pub(crate) mod utils;

pub use utils::is_running_in_vm;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be the first plugin registered, same as soplos-welcome-3.0:
        // it needs to intercept and exit a second launch before anything
        // else runs.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            system_theme::get_system_theme,
            commands::webapps::list_webapps,
            commands::webapps::create_webapp,
            commands::webapps::update_webapp,
            commands::webapps::delete_webapp,
            commands::webapps::run_webapp,
            commands::webapps::read_icon_preview,
            commands::webapps::pick_icon_file,
            commands::browsers::list_browsers,
            commands::favicon::download_favicon,
            commands::environment::get_environment_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Soplos WebApp Manager");
}
