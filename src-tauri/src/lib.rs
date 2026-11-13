//! Central DevOps Kit Tauri 入口。
//!
//! - `setup`：托盘、开机自启、更新插件
//! - `on_window_event`：关闭到托盘
//! - `invoke_handler`：前端 IPC（实现见 `commands`）

mod commands;
mod gitlab;
mod platform;
mod ports;
mod settings;
mod tray;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&'static str>>,
        ))
        .setup(|app| {
            #[cfg(desktop)]
            {
                if let Err(e) = app
                    .handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())
                {
                    eprintln!("[CentralDevOpsKit] updater plugin failed: {e}");
                }
            }
            if let Err(e) = tray::setup_tray(app.handle()) {
                eprintln!("[CentralDevOpsKit] tray setup failed: {e}");
            }
            if let Ok(saved) = settings::load_settings() {
                let _ = settings::sync_launch_at_login(app.handle(), saved.general.launch_at_login);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() != "main" {
                    return;
                }
                api.prevent_close();
                if settings::close_to_tray_enabled() {
                    let _ = window.hide();
                } else {
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::runtime_settings_exists,
            commands::load_runtime_settings,
            commands::save_runtime_settings,
            commands::initialize_runtime_settings,
            commands::exit_app,
            commands::gitlab_load_variables,
            commands::ports_lookup,
            commands::ports_kill,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = &event {
                tray::show_main_window(app_handle);
            }
        });
}
