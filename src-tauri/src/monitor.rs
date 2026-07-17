//! 将 Windows 全屏、最大化、电池和显示器电源状态转换为自动暂停原因。

use crate::commands::{self, RuntimeState};
use crate::model::PauseReason;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;
use tauri::{App, Manager};

/// 启动低频系统状态监视器；进程退出时线程随进程结束。
pub fn install(app: &App) {
    let app = app.handle().clone();
    let (display_sender, display_receiver) = mpsc::channel();
    #[cfg(windows)]
    thread::spawn(move || display_power_listener(display_sender));
    thread::spawn(move || monitor_loop(app, display_receiver));
}

fn monitor_loop(app: tauri::AppHandle, display_receiver: Receiver<bool>) {
    let mut display_sleeping = false;
    let mut previous = None;
    loop {
        let _ = commands::refresh_displays(app.clone(), app.state::<RuntimeState>());
        while let Ok(value) = display_receiver.try_recv() {
            display_sleeping = value;
        }
        let Ok(snapshot) = commands::bootstrap(app.state::<RuntimeState>()) else {
            thread::sleep(Duration::from_secs(1));
            continue;
        };
        let foreground_display = foreground_display_id();
        let fullscreen = is_foreground_fullscreen();
        let maximized = is_foreground_maximized();
        let per_target = snapshot
            .playback
            .display_assignments
            .iter()
            .map(|assignment| {
                let covered = foreground_display
                    .as_ref()
                    .is_some_and(|id| assignment.display_ids.contains(id));
                (
                    assignment.target_id.clone(),
                    snapshot.settings.pause_on_fullscreen && fullscreen && covered,
                    snapshot.settings.pause_on_maximized && maximized && covered,
                )
            })
            .collect::<Vec<_>>();
        let has_targets = !per_target.is_empty();
        let desired = (
            snapshot.playback.active_id.clone(),
            !has_targets && snapshot.settings.pause_on_fullscreen && fullscreen,
            !has_targets && snapshot.settings.pause_on_maximized && maximized,
            snapshot.settings.pause_on_battery && is_on_battery(),
            snapshot.settings.pause_on_display_sleep && display_sleeping,
            per_target,
        );
        if previous.as_ref() != Some(&desired) {
            let mut succeeded = commands::set_automatic_pause(
                app.clone(),
                app.state::<RuntimeState>(),
                PauseReason::Fullscreen,
                desired.1,
            )
            .is_ok();
            succeeded &= commands::set_automatic_pause(
                app.clone(),
                app.state::<RuntimeState>(),
                PauseReason::Battery,
                desired.3,
            )
            .is_ok();
            succeeded &= commands::set_automatic_pause(
                app.clone(),
                app.state::<RuntimeState>(),
                PauseReason::DisplaySleep,
                desired.4,
            )
            .is_ok();
            succeeded &= commands::set_automatic_pause(
                app.clone(),
                app.state::<RuntimeState>(),
                PauseReason::Maximized,
                desired.2,
            )
            .is_ok();
            for (target_id, fullscreen, maximized) in &desired.5 {
                succeeded &= commands::set_target_automatic_pause(
                    app.clone(),
                    app.state::<RuntimeState>(),
                    target_id,
                    PauseReason::Fullscreen,
                    *fullscreen,
                )
                .is_ok();
                succeeded &= commands::set_target_automatic_pause(
                    app.clone(),
                    app.state::<RuntimeState>(),
                    target_id,
                    PauseReason::Maximized,
                    *maximized,
                )
                .is_ok();
            }
            if succeeded {
                previous = Some(desired);
            }
        }
        let _ = commands::sync_clone_players(app.state::<RuntimeState>());
        thread::sleep(Duration::from_secs(1));
    }
}

#[cfg(windows)]
fn is_on_battery() -> bool {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status).is_ok() && status.ACLineStatus == 0 }
}

#[cfg(not(windows))]
fn is_on_battery() -> bool {
    false
}

#[cfg(windows)]
fn is_foreground_fullscreen() -> bool {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
    };

    unsafe {
        let window = GetForegroundWindow();
        if window.is_invalid() {
            return false;
        }
        let mut process_id = 0;
        GetWindowThreadProcessId(window, Some(&mut process_id));
        if process_id == std::process::id() {
            return false;
        }
        let monitor = MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST);
        let mut monitor_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let mut window_rect = RECT::default();
        if GetMonitorInfoW(monitor, &mut monitor_info).as_bool()
            && GetWindowRect(window, &mut window_rect).is_ok()
        {
            let monitor_rect = monitor_info.rcMonitor;
            window_rect.left <= monitor_rect.left
                && window_rect.top <= monitor_rect.top
                && window_rect.right >= monitor_rect.right
                && window_rect.bottom >= monitor_rect.bottom
        } else {
            false
        }
    }
}

#[cfg(not(windows))]
fn is_foreground_fullscreen() -> bool {
    false
}

#[cfg(windows)]
fn is_foreground_maximized() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, IsZoomed,
    };

    unsafe {
        let window = GetForegroundWindow();
        if window.is_invalid() {
            return false;
        }
        let mut process_id = 0;
        GetWindowThreadProcessId(window, Some(&mut process_id));
        process_id != std::process::id() && IsZoomed(window).as_bool()
    }
}

#[cfg(not(windows))]
fn is_foreground_maximized() -> bool {
    false
}

#[cfg(windows)]
fn foreground_display_id() -> Option<String> {
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MONITORINFOEXW, MonitorFromWindow,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    unsafe {
        let window = GetForegroundWindow();
        if window.is_invalid() {
            return None;
        }
        let mut process_id = 0;
        GetWindowThreadProcessId(window, Some(&mut process_id));
        if process_id == std::process::id() {
            return None;
        }
        let monitor = MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFOEXW {
            monitorInfo: MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
                ..Default::default()
            },
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info.monitorInfo).as_bool() {
            return None;
        }
        let length = info
            .szDevice
            .iter()
            .position(|character| *character == 0)
            .unwrap_or(info.szDevice.len());
        Some(String::from_utf16_lossy(&info.szDevice[..length]))
    }
}

#[cfg(not(windows))]
fn foreground_display_id() -> Option<String> {
    None
}

#[cfg(windows)]
fn display_power_listener(sender: Sender<bool>) {
    use std::sync::OnceLock;
    use windows::Win32::Foundation::{HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Power::{POWERBROADCAST_SETTING, RegisterPowerSettingNotification};
    use windows::Win32::System::SystemServices::GUID_CONSOLE_DISPLAY_STATE;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DEVICE_NOTIFY_WINDOW_HANDLE, DefWindowProcW, DispatchMessageW,
        GetMessageW, HWND_MESSAGE, MSG, PBT_POWERSETTINGCHANGE, RegisterClassW, WINDOW_EX_STYLE,
        WINDOW_STYLE, WM_POWERBROADCAST, WNDCLASSW,
    };
    use windows::core::w;

    static DISPLAY_SENDER: OnceLock<Sender<bool>> = OnceLock::new();

    unsafe extern "system" fn window_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if message == WM_POWERBROADCAST && wparam.0 as u32 == PBT_POWERSETTINGCHANGE {
            let setting = unsafe { &*(lparam.0 as *const POWERBROADCAST_SETTING) };
            if setting.PowerSetting == GUID_CONSOLE_DISPLAY_STATE {
                let value = setting.Data[0];
                if let Some(sender) = DISPLAY_SENDER.get() {
                    let _ = sender.send(value == 0);
                }
            }
        }
        unsafe { DefWindowProcW(window, message, wparam, lparam) }
    }

    let _ = DISPLAY_SENDER.set(sender);
    unsafe {
        let Ok(module) = GetModuleHandleW(None) else {
            return;
        };
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: HINSTANCE(module.0),
            lpszClassName: w!("WallDisplayPowerListener"),
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return;
        }
        let Ok(window) = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class.lpszClassName,
            w!("WallDisplayPowerListener"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(class.hInstance),
            None,
        ) else {
            return;
        };
        if RegisterPowerSettingNotification(
            HANDLE(window.0),
            &GUID_CONSOLE_DISPLAY_STATE,
            DEVICE_NOTIFY_WINDOW_HANDLE,
        )
        .is_err()
        {
            return;
        }
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            DispatchMessageW(&message);
        }
    }
}

#[cfg(not(windows))]
fn display_power_listener(_: Sender<bool>) {}
