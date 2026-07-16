//! 创建 Windows 托盘菜单并使其始终反映共享 AppSnapshot。

use crate::commands::{self, RuntimeState};
use crate::model::{AppSnapshot, PlaybackStatus};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, Manager};

pub struct TrayMenuState {
    current: MenuItem<tauri::Wry>,
    pause: MenuItem<tauri::Wry>,
    mute: CheckMenuItem<tauri::Wry>,
    stop: MenuItem<tauri::Wry>,
    autostart: CheckMenuItem<tauri::Wry>,
}

impl TrayMenuState {
    /// 更新动态文本、启用状态和复选状态。
    pub fn update(&self, snapshot: &AppSnapshot) {
        let active = snapshot
            .playback
            .active_id
            .as_ref()
            .and_then(|id| snapshot.library.iter().find(|item| &item.id == id));
        let running = active.is_some();
        let _ = self.current.set_text(
            active
                .map(|item| format!("当前壁纸：{}", item.name))
                .unwrap_or_else(|| "未运行壁纸".to_owned()),
        );
        let _ = self.pause.set_enabled(running);
        let _ = self
            .pause
            .set_text(if snapshot.playback.status == PlaybackStatus::Paused {
                "继续壁纸"
            } else {
                "暂停壁纸"
            });
        let _ = self.mute.set_enabled(running);
        let _ = self.mute.set_checked(snapshot.playback.muted);
        let _ = self.stop.set_enabled(running);
        let _ = self.autostart.set_checked(snapshot.settings.auto_start);
    }
}

/// 安装托盘图标、动态菜单和单击打开行为。
pub fn install(app: &App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "打开 Wall", true, None::<&str>)?;
    let current = MenuItem::with_id(app, "current", "未运行壁纸", false, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "暂停壁纸", false, None::<&str>)?;
    let mute = CheckMenuItem::with_id(app, "mute", "静音", false, true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "停止壁纸", false, None::<&str>)?;
    let autostart =
        CheckMenuItem::with_id(app, "autostart", "开机启动", true, false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 Wall", true, None::<&str>)?;
    let separator_one = PredefinedMenuItem::separator(app)?;
    let separator_two = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &current,
            &separator_one,
            &pause,
            &mute,
            &stop,
            &separator_two,
            &autostart,
            &quit,
        ],
    )?;

    let state = TrayMenuState {
        current,
        pause,
        mute,
        stop,
        autostart,
    };
    if let Ok(snapshot) = commands::bootstrap(app.state::<RuntimeState>()) {
        state.update(&snapshot);
    }
    app.manage(state);

    let icon = app.default_window_icon().cloned();
    let mut builder = TrayIconBuilder::with_id("wall")
        .menu(&menu)
        .tooltip("Wall")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "pause" => {
                let _ = commands::toggle_pause(app.clone(), app.state::<RuntimeState>());
            }
            "mute" => {
                if let Ok(snapshot) = commands::bootstrap(app.state::<RuntimeState>()) {
                    let _ = commands::set_muted(
                        app.clone(),
                        app.state::<RuntimeState>(),
                        !snapshot.playback.muted,
                    );
                }
            }
            "stop" => {
                let _ = commands::stop(app.clone(), app.state::<RuntimeState>());
            }
            "autostart" => {
                if let Ok(snapshot) = commands::bootstrap(app.state::<RuntimeState>()) {
                    let mut settings = snapshot.settings;
                    settings.auto_start = !settings.auto_start;
                    let _ = commands::update_settings(
                        app.clone(),
                        app.state::<RuntimeState>(),
                        settings,
                    );
                }
            }
            "quit" => commands::quit(app.clone(), app.state::<RuntimeState>()),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = icon {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
