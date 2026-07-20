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

fn restore_manual_pause(assignment: &model::DisplayAssignment) -> bool {
    assignment
        .pause_reasons
        .contains(&model::PauseReason::Manual)
}

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
            {
                if snapshot.playback.display_assignments.is_empty() {
                    if let Some(media_id) = snapshot.playback.active_id
                        && let Err(error) = commands::play(
                            app.handle().clone(),
                            app.state::<commands::RuntimeState>(),
                            media_id,
                        )
                    {
                        app.state::<commands::RuntimeState>()
                            .log_error("restore", &error.message);
                    }
                } else {
                    for assignment in snapshot.playback.display_assignments {
                        let target_id = assignment.target_id.clone();
                        let muted = assignment.muted;
                        let manual_pause = restore_manual_pause(&assignment);
                        let restored = commands::set_display_layout(
                            app.handle().clone(),
                            app.state::<commands::RuntimeState>(),
                            assignment.mode,
                            assignment.display_ids,
                        )
                        .and_then(|_| {
                            commands::play(
                                app.handle().clone(),
                                app.state::<commands::RuntimeState>(),
                                assignment.wallpaper_id,
                            )
                        })
                        .and_then(|_| {
                            commands::set_target_muted(
                                app.handle().clone(),
                                app.state::<commands::RuntimeState>(),
                                target_id.clone(),
                                muted,
                            )
                        })
                        .and_then(|snapshot| {
                            if manual_pause {
                                commands::toggle_target_pause(
                                    app.handle().clone(),
                                    app.state::<commands::RuntimeState>(),
                                    target_id,
                                )
                            } else {
                                Ok(snapshot)
                            }
                        });
                        if let Err(error) = restored {
                            app.state::<commands::RuntimeState>()
                                .log_error("restore", &error.message);
                        }
                    }
                }
            }
            monitor::install(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::import_media,
            commands::create_category,
            commands::rename_category,
            commands::delete_category,
            commands::set_category_membership,
            commands::set_display_layout,
            commands::set_wallpaper_settings,
            commands::remove_media,
            commands::remove_media_batch,
            commands::scan_library,
            commands::remove_missing_media,
            commands::relocate_media,
            commands::play,
            commands::toggle_pause,
            commands::toggle_target_pause,
            commands::toggle_media_pause,
            commands::stop,
            commands::set_muted,
            commands::set_target_muted,
            commands::stop_target,
            commands::stop_media,
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

#[cfg(test)]
mod tests {
    use super::restore_manual_pause;
    use crate::model::{DisplayAssignment, DisplayMode, PauseReason, PlaybackStatus};

    #[test]
    fn restart_only_restores_the_user_owned_pause_reason() {
        let assignment = DisplayAssignment {
            target_id: "display-primary".to_owned(),
            mode: DisplayMode::Independent,
            display_ids: vec!["primary".to_owned()],
            wallpaper_id: "wallpaper".to_owned(),
            status: PlaybackStatus::Paused,
            muted: true,
            volume: 0,
            pause_reasons: vec![PauseReason::Fullscreen],
        };
        assert!(!restore_manual_pause(&assignment));

        let mut manual = assignment;
        manual.pause_reasons.push(PauseReason::Manual);
        assert!(restore_manual_pause(&manual));
    }
}
