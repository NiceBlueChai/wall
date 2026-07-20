//! 创建 Windows 托盘菜单并使其始终反映共享 AppSnapshot。

use crate::commands::{self, RuntimeState};
use crate::model::{AppSnapshot, DisplayMode, PlaybackStatus};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, Manager};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TrayIconState {
    Active,
    Paused,
    Idle,
}

impl TrayIconState {
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Active => include_bytes!("../icons/tray/active.rgba"),
            Self::Paused => include_bytes!("../icons/tray/paused.rgba"),
            Self::Idle => include_bytes!("../icons/tray/idle.rgba"),
        }
    }

    fn image(self) -> tauri::image::Image<'static> {
        tauri::image::Image::new(self.bytes(), 24, 24)
    }
}

fn tray_icon_state(snapshot: &AppSnapshot) -> TrayIconState {
    let mut statuses = snapshot
        .playback
        .display_assignments
        .iter()
        .map(|assignment| assignment.status);
    if statuses
        .clone()
        .any(|status| status == PlaybackStatus::Playing)
    {
        TrayIconState::Active
    } else if statuses.any(|status| status == PlaybackStatus::Paused) {
        TrayIconState::Paused
    } else {
        match snapshot.playback.status {
            PlaybackStatus::Playing => TrayIconState::Active,
            PlaybackStatus::Paused => TrayIconState::Paused,
            PlaybackStatus::Idle | PlaybackStatus::Error => TrayIconState::Idle,
        }
    }
}

pub struct TrayMenuState {
    app: tauri::AppHandle,
    current: MenuItem<tauri::Wry>,
    pause: MenuItem<tauri::Wry>,
    mute: CheckMenuItem<tauri::Wry>,
    stop: MenuItem<tauri::Wry>,
    autostart: CheckMenuItem<tauri::Wry>,
    targets: Submenu<tauri::Wry>,
}

impl TrayMenuState {
    /// 更新动态文本、启用状态和复选状态。
    pub fn update(&self, snapshot: &AppSnapshot) {
        if let Some(tray) = self.app.tray_by_id("wall") {
            let _ = tray.set_icon(Some(tray_icon_state(snapshot).image()));
        }
        let active = snapshot
            .playback
            .active_id
            .as_ref()
            .and_then(|id| snapshot.library.iter().find(|item| &item.id == id));
        let running = !snapshot.playback.display_assignments.is_empty() || active.is_some();
        let assignment_count = snapshot.playback.display_assignments.len();
        let paused_count = snapshot
            .playback
            .display_assignments
            .iter()
            .filter(|assignment| assignment.status == PlaybackStatus::Paused)
            .count();
        let _ = self.current.set_text(if assignment_count > 1 {
            if paused_count == assignment_count {
                format!("{assignment_count} 个显示目标 · 全部暂停")
            } else if paused_count > 0 {
                format!("{assignment_count} 个显示目标 · 部分暂停")
            } else {
                format!("{assignment_count} 个显示目标运行中")
            }
        } else {
            active
                .map(|item| format!("当前壁纸：{}", item.name))
                .unwrap_or_else(|| "全部屏幕空闲".to_owned())
        });
        let _ = self.pause.set_enabled(running);
        let _ = self
            .pause
            .set_text(if snapshot.playback.status == PlaybackStatus::Paused {
                "全部屏幕继续"
            } else {
                "全部屏幕暂停"
            });
        let _ = self.mute.set_enabled(running);
        let _ = self.mute.set_checked(snapshot.playback.muted);
        let _ = self.stop.set_enabled(running);
        let _ = self.autostart.set_checked(snapshot.settings.auto_start);
        let _ = self
            .targets
            .set_enabled(!snapshot.playback.display_assignments.is_empty());
        if let Ok(items) = self.targets.items() {
            for item in items {
                let _ = self.targets.remove(&item);
            }
        }
        for assignment in &snapshot.playback.display_assignments {
            let names = assignment
                .display_ids
                .iter()
                .filter_map(|id| snapshot.displays.iter().find(|display| &display.id == id))
                .map(|display| display.name.as_str())
                .collect::<Vec<_>>()
                .join(" + ");
            let label = match assignment.mode {
                DisplayMode::Independent => names,
                DisplayMode::Clone => format!("复制组 · {names}"),
                DisplayMode::Span => format!("铺展组 · {names}"),
            };
            let pause = MenuItem::with_id(
                &self.app,
                format!("target-pause:{}", assignment.target_id),
                if assignment.status == PlaybackStatus::Paused {
                    "继续"
                } else {
                    "暂停"
                },
                true,
                None::<&str>,
            );
            let mute = CheckMenuItem::with_id(
                &self.app,
                format!("target-mute:{}", assignment.target_id),
                "静音",
                true,
                assignment.muted,
                None::<&str>,
            );
            let stop = MenuItem::with_id(
                &self.app,
                format!("target-stop:{}", assignment.target_id),
                "停止",
                true,
                None::<&str>,
            );
            if let (Ok(pause), Ok(mute), Ok(stop)) = (pause, mute, stop)
                && let Ok(submenu) = Submenu::with_id_and_items(
                    &self.app,
                    format!("target:{}", assignment.target_id),
                    label,
                    true,
                    &[&pause, &mute, &stop],
                )
            {
                let _ = self.targets.append(&submenu);
            }
        }
    }
}

/// 安装托盘图标、动态菜单和单击打开行为。
pub fn install(app: &App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "打开 Wall", true, None::<&str>)?;
    let current = MenuItem::with_id(app, "current", "未运行壁纸", false, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "全部屏幕暂停", false, None::<&str>)?;
    let mute = CheckMenuItem::with_id(app, "mute", "全部屏幕静音", false, true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "全部屏幕停止", false, None::<&str>)?;
    let targets = Submenu::with_id(app, "targets", "按显示器操作", false)?;
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
            &targets,
            &separator_two,
            &autostart,
            &quit,
        ],
    )?;

    let snapshot = commands::bootstrap(app.state::<RuntimeState>()).ok();
    let icon_state = snapshot
        .as_ref()
        .map(tray_icon_state)
        .unwrap_or(TrayIconState::Idle);
    let builder = TrayIconBuilder::with_id("wall")
        .icon(icon_state.image())
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
            id if id.starts_with("target-pause:") => {
                let target_id = id.trim_start_matches("target-pause:").to_owned();
                let _ = commands::toggle_target_pause(
                    app.clone(),
                    app.state::<RuntimeState>(),
                    target_id,
                );
            }
            id if id.starts_with("target-mute:") => {
                let target_id = id.trim_start_matches("target-mute:").to_owned();
                if let Ok(snapshot) = commands::bootstrap(app.state::<RuntimeState>())
                    && let Some(assignment) = snapshot
                        .playback
                        .display_assignments
                        .iter()
                        .find(|assignment| assignment.target_id == target_id)
                {
                    let _ = commands::set_target_muted(
                        app.clone(),
                        app.state::<RuntimeState>(),
                        target_id,
                        !assignment.muted,
                    );
                }
            }
            id if id.starts_with("target-stop:") => {
                let target_id = id.trim_start_matches("target-stop:").to_owned();
                let _ = commands::stop_target(app.clone(), app.state::<RuntimeState>(), target_id);
            }
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
    builder.build(app)?;

    let state = TrayMenuState {
        app: app.handle().clone(),
        current,
        pause,
        mute,
        stop,
        autostart,
        targets,
    };
    if let Some(snapshot) = snapshot {
        state.update(&snapshot);
    }
    app.manage(state);
    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::{TrayIconState, tray_icon_state};
    use crate::model::{AppSnapshot, DisplayAssignment, DisplayMode, PlaybackStatus};

    #[test]
    fn tray_icon_tracks_idle_paused_and_active_playback() {
        let mut snapshot = AppSnapshot::default();
        assert_eq!(tray_icon_state(&snapshot), TrayIconState::Idle);

        snapshot.playback.active_id = Some("wallpaper".to_owned());
        snapshot.playback.status = PlaybackStatus::Paused;
        snapshot.playback.display_assignments = vec![assignment("left", PlaybackStatus::Paused)];
        assert_eq!(tray_icon_state(&snapshot), TrayIconState::Paused);

        snapshot
            .playback
            .display_assignments
            .push(assignment("right", PlaybackStatus::Playing));
        assert_eq!(tray_icon_state(&snapshot), TrayIconState::Active);
        assert_eq!(TrayIconState::Active.bytes().len(), 24 * 24 * 4);
        assert_eq!(TrayIconState::Paused.bytes().len(), 24 * 24 * 4);
        assert_eq!(TrayIconState::Idle.bytes().len(), 24 * 24 * 4);
        assert_eq!(opaque_bounds(TrayIconState::Active.bytes()), (1, 1, 22, 22));
        assert_eq!(opaque_bounds(TrayIconState::Paused.bytes()), (1, 1, 22, 22));
        assert_eq!(opaque_bounds(TrayIconState::Idle.bytes()), (1, 1, 22, 22));
    }

    fn opaque_bounds(bytes: &[u8]) -> (usize, usize, usize, usize) {
        let mut min_x = 24;
        let mut min_y = 24;
        let mut max_x = 0;
        let mut max_y = 0;
        for (index, pixel) in bytes.chunks_exact(4).enumerate() {
            if pixel[3] <= 16 {
                continue;
            }
            let x = index % 24;
            let y = index / 24;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        (min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
    }

    fn assignment(target_id: &str, status: PlaybackStatus) -> DisplayAssignment {
        DisplayAssignment {
            target_id: target_id.to_owned(),
            mode: DisplayMode::Independent,
            display_ids: vec![target_id.to_owned()],
            wallpaper_id: "wallpaper".to_owned(),
            status,
            muted: true,
            volume: 0,
            pause_reasons: Vec::new(),
        }
    }
}
