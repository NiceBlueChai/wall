//! Wall 的 Tauri 库入口和命令注册中心。

pub mod commands;
pub mod core;
pub mod media;
pub mod model;
pub mod monitor;
pub mod player;
pub mod storage;
pub mod tray;

use tauri::Manager as _;

/// 启动 Wall 桌面应用。
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(commands::RuntimeState::load(app)?);
            tray::install(app)?;
            monitor::install(app);
            if let Some(window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event
                        && let Ok(snapshot) =
                            commands::bootstrap(app_handle.state::<commands::RuntimeState>())
                        && snapshot.settings.close_to_tray
                    {
                        api.prevent_close();
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                });
            }
            if let Ok(snapshot) = commands::bootstrap(app.state::<commands::RuntimeState>())
                && snapshot.settings.restore_last_wallpaper
                && let Some(media_id) = snapshot.playback.active_id
            {
                if let Err(error) = commands::play(
                    app.handle().clone(),
                    app.state::<commands::RuntimeState>(),
                    media_id,
                ) {
                    app.state::<commands::RuntimeState>()
                        .log_error("restore", &error.message);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::import_media,
            commands::remove_media,
            commands::relocate_media,
            commands::play,
            commands::toggle_pause,
            commands::stop,
            commands::set_muted,
            commands::set_volume,
            commands::set_scale_mode,
            commands::update_settings,
            commands::open_media_folder,
            commands::open_logs,
            commands::open_license,
            commands::open_project_homepage,
            commands::quit,
        ])
        .run(tauri::generate_context!())
        .expect("Wall 启动失败");
}
