//! 启动并通过命名管道控制随应用分发的 mpv。

use crate::model::{AppSettings, MediaKind};
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use thiserror::Error;

/// 构造不读取用户配置、无交互并创建独立窗口的 mpv 参数。
pub fn build_mpv_arguments(
    media_path: &Path,
    kind: MediaKind,
    window_title: &str,
    screen_width: i32,
    screen_height: i32,
    pipe_name: &str,
    settings: &AppSettings,
) -> Vec<String> {
    let mut arguments = vec![
        "--no-config".to_owned(),
        "--terminal=no".to_owned(),
        "--force-window=immediate".to_owned(),
        "--idle=no".to_owned(),
        "--loop-file=inf".to_owned(),
        "--no-border".to_owned(),
        "--no-osc".to_owned(),
        "--no-input-default-bindings".to_owned(),
        "--input-cursor=no".to_owned(),
        "--input-vo-keyboard=no".to_owned(),
        "--background-color=#000000".to_owned(),
        format!("--title={window_title}"),
        format!("--geometry={screen_width}x{screen_height}+0+0"),
        format!("--input-ipc-server={pipe_name}"),
        format!(
            "--hwdec={}",
            if settings.hardware_decoding {
                "auto-safe"
            } else {
                "no"
            }
        ),
        format!(
            "--mute={}",
            if settings.default_muted { "yes" } else { "no" }
        ),
        format!("--volume={}", settings.volume),
        format!("--vf=fps={}", settings.frame_rate),
    ];
    arguments.extend(settings.scale_mode.mpv_arguments().map(str::to_owned));
    if kind == MediaKind::Image {
        arguments.push("--image-display-duration=inf".to_owned());
    }
    arguments.push(media_path.to_string_lossy().into_owned());
    arguments
}

#[derive(Debug, Error)]
pub enum PlayerError {
    #[error("未找到随 Wall 分发的 mpv.exe")]
    MissingBinary,
    #[error("无法找到 Windows 桌面宿主窗口")]
    MissingDesktopHost,
    #[error("mpv 窗口未在 10 秒内创建")]
    WindowTimeout,
    #[error("mpv 在创建播放窗口前退出，退出码：{0:?}")]
    EarlyExit(Option<i32>),
    #[error("无法把 mpv 窗口嵌入桌面：{0}")]
    Embedding(String),
    #[error("无法启动或控制 mpv：{0}")]
    Io(#[from] std::io::Error),
}

#[derive(Default)]
pub struct MpvPlayer {
    child: Option<Child>,
    pipe_name: Option<String>,
    player_window: Option<isize>,
    order_stop: Option<Arc<AtomicBool>>,
    order_thread: Option<JoinHandle<()>>,
}

impl MpvPlayer {
    /// 启动新的 mpv 进程并嵌入 Windows 桌面图标层后方。
    pub fn play(
        &mut self,
        binary: &Path,
        media_path: &Path,
        kind: MediaKind,
        settings: &AppSettings,
    ) -> Result<(), PlayerError> {
        if !binary.is_file() {
            return Err(PlayerError::MissingBinary);
        }
        self.stop();
        let desktop = desktop_host_window().ok_or(PlayerError::MissingDesktopHost)?;
        let (screen_width, screen_height) = primary_screen_size();
        let window_title = format!("wall-wallpaper-{}", uuid::Uuid::new_v4());
        let pipe_name = format!(r"\\.\pipe\wall-mpv-{}", std::process::id());
        let arguments = build_mpv_arguments(
            media_path,
            kind,
            &window_title,
            screen_width,
            screen_height,
            &pipe_name,
            settings,
        );
        let mut command = Command::new(binary);
        command
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command.spawn()?;
        let player_window = match wait_for_player_window(&window_title, &mut child) {
            Ok(window) => window,
            Err(error) => {
                terminate_child(&mut child);
                return Err(error);
            }
        };
        if let Err(error) = embed_player_window(player_window, desktop, screen_width, screen_height)
        {
            terminate_child(&mut child);
            return Err(error);
        }
        if desktop.raised {
            let (stop, handle) = maintain_window_order(player_window, desktop.shell_view);
            self.order_stop = Some(stop);
            self.order_thread = Some(handle);
        }
        self.child = Some(child);
        self.pipe_name = Some(pipe_name);
        self.player_window = Some(player_window);
        Ok(())
    }

    /// 暂停或继续当前 mpv 进程。
    pub fn set_paused(&self, paused: bool) -> Result<(), PlayerError> {
        self.send(json!({ "command": ["set_property", "pause", paused] }))
    }

    /// 设置 mpv 静音状态。
    pub fn set_muted(&self, muted: bool) -> Result<(), PlayerError> {
        self.send(json!({ "command": ["set_property", "mute", muted] }))
    }

    /// 设置 mpv 音量。
    pub fn set_volume(&self, volume: u8) -> Result<(), PlayerError> {
        self.send(json!({ "command": ["set_property", "volume", volume] }))
    }

    /// 动态更新保持比例和裁切参数。
    pub fn set_scale_mode(&self, settings: &AppSettings) -> Result<(), PlayerError> {
        let [keep_aspect, panscan] = settings.scale_mode.mpv_arguments();
        self.send(
            json!({ "command": ["set_property", "keepaspect", keep_aspect.ends_with("yes")] }),
        )?;
        let panscan = panscan
            .rsplit('=')
            .next()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.0);
        self.send(json!({ "command": ["set_property", "panscan", panscan] }))
    }

    /// 停止当前 mpv 进程；管道不可用时直接终止子进程。
    pub fn stop(&mut self) {
        if let Some(stop) = self.order_stop.take() {
            stop.store(true, Ordering::Release);
        }
        if let Some(handle) = self.order_thread.take() {
            let _ = handle.join();
        }
        if self.child.is_some() {
            let _ = self.send(json!({ "command": ["quit"] }));
        }
        if let Some(child) = self.child.as_mut() {
            thread::sleep(Duration::from_millis(80));
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        self.child = None;
        self.pipe_name = None;
        self.player_window = None;
    }

    fn send(&self, command: serde_json::Value) -> Result<(), PlayerError> {
        let pipe_name = self
            .pipe_name
            .as_ref()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotConnected, "mpv 尚未运行"))?;
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match OpenOptions::new().write(true).open(pipe_name) {
                Ok(mut pipe) => {
                    serde_json::to_writer(&mut pipe, &command).map_err(std::io::Error::other)?;
                    pipe.write_all(b"\n")?;
                    return Ok(());
                }
                Err(_) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(40));
                }
                Err(error) => return Err(error.into()),
            }
        }
    }
}

impl Drop for MpvPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn embedding_flags(raised_desktop: bool) -> u32 {
    const SWP_NO_Z_ORDER: u32 = 0x0004;
    const SWP_NO_ACTIVATE: u32 = 0x0010;
    const SWP_FRAME_CHANGED: u32 = 0x0020;
    const SWP_SHOW_WINDOW: u32 = 0x0040;

    let flags = SWP_NO_ACTIVATE | SWP_FRAME_CHANGED | SWP_SHOW_WINDOW;
    if raised_desktop {
        flags
    } else {
        flags | SWP_NO_Z_ORDER
    }
}

#[derive(Clone, Copy)]
struct DesktopHost {
    host: isize,
    shell_view: isize,
    raised: bool,
}

#[cfg(windows)]
fn desktop_host_window() -> Option<DesktopHost> {
    use windows::Win32::Foundation::{HWND, LPARAM, TRUE, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, FindWindowExW, FindWindowW, GWL_EXSTYLE, GetWindowLongPtrW, SMTO_NORMAL,
        SendMessageTimeoutW, WS_EX_NOREDIRECTIONBITMAP,
    };
    use windows::core::{BOOL, w};

    unsafe extern "system" fn find_worker(window: HWND, data: LPARAM) -> BOOL {
        unsafe {
            if FindWindowExW(Some(window), None, w!("SHELLDLL_DefView"), None).is_ok()
                && let Ok(worker) = FindWindowExW(None, Some(window), w!("WorkerW"), None)
            {
                *(data.0 as *mut HWND) = worker;
                return BOOL(0);
            }
        }
        TRUE
    }

    unsafe {
        let progman = FindWindowW(w!("Progman"), None).ok()?;
        let _ = SendMessageTimeoutW(
            progman,
            0x052C,
            WPARAM(0xD),
            LPARAM(1),
            SMTO_NORMAL,
            1000,
            None,
        );
        let shell_view = FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), None)
            .ok()
            .unwrap_or_default();
        let extended_style = GetWindowLongPtrW(progman, GWL_EXSTYLE) as u32;
        let raised = extended_style & WS_EX_NOREDIRECTIONBITMAP.0 != 0;
        let mut worker = HWND::default();
        if !raised {
            let _ = EnumWindows(
                Some(find_worker),
                LPARAM((&mut worker as *mut HWND) as isize),
            );
        }
        let host = if raised || worker.is_invalid() {
            progman
        } else {
            worker
        };
        if raised && shell_view.is_invalid() {
            return None;
        }
        Some(DesktopHost {
            host: host.0 as isize,
            shell_view: shell_view.0 as isize,
            raised,
        })
    }
}

#[cfg(not(windows))]
fn desktop_host_window() -> Option<DesktopHost> {
    None
}

#[cfg(windows)]
fn primary_screen_size() -> (i32, i32) {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
}

#[cfg(not(windows))]
fn primary_screen_size() -> (i32, i32) {
    (1920, 1080)
}

#[cfg(windows)]
fn wait_for_player_window(title: &str, child: &mut Child) -> Result<isize, PlayerError> {
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    use windows::core::PCWSTR;

    let wide_title: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(window) = unsafe { FindWindowW(None, PCWSTR(wide_title.as_ptr())) } {
            return Ok(window.0 as isize);
        }
        if let Some(status) = child.try_wait()? {
            return Err(PlayerError::EarlyExit(status.code()));
        }
        if Instant::now() >= deadline {
            return Err(PlayerError::WindowTimeout);
        }
        thread::sleep(Duration::from_millis(40));
    }
}

#[cfg(not(windows))]
fn wait_for_player_window(_title: &str, _child: &mut Child) -> Result<isize, PlayerError> {
    Err(PlayerError::MissingDesktopHost)
}

#[cfg(windows)]
fn embed_player_window(
    player_window: isize,
    desktop: DesktopHost,
    width: i32,
    height: i32,
) -> Result<(), PlayerError> {
    use core::ffi::c_void;
    use windows::Win32::Foundation::{COLORREF, HWND};
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GWL_STYLE, GetParent, GetWindowLongPtrW, LWA_ALPHA, SET_WINDOW_POS_FLAGS,
        SetLayeredWindowAttributes, SetParent, SetWindowLongPtrW, SetWindowPos, WS_CHILD,
        WS_EX_LAYERED, WS_POPUP,
    };

    let player = HWND(player_window as *mut c_void);
    let host = HWND(desktop.host as *mut c_void);
    let shell_view = HWND(desktop.shell_view as *mut c_void);
    unsafe {
        let style = GetWindowLongPtrW(player, GWL_STYLE);
        let child_style = (style & !(WS_POPUP.0 as isize)) | WS_CHILD.0 as isize;
        SetWindowLongPtrW(player, GWL_STYLE, child_style);
        if desktop.raised {
            let extended_style = GetWindowLongPtrW(player, GWL_EXSTYLE);
            SetWindowLongPtrW(
                player,
                GWL_EXSTYLE,
                extended_style | WS_EX_LAYERED.0 as isize,
            );
            SetLayeredWindowAttributes(player, COLORREF(0), 255, LWA_ALPHA)
                .map_err(|error| PlayerError::Embedding(error.to_string()))?;
        }
        let _ = SetParent(player, Some(host));
        let parent =
            GetParent(player).map_err(|error| PlayerError::Embedding(error.to_string()))?;
        if parent != host {
            return Err(PlayerError::Embedding("SetParent 未生效".to_owned()));
        }
        let insert_after = desktop.raised.then_some(shell_view);
        SetWindowPos(
            player,
            insert_after,
            0,
            0,
            width,
            height,
            SET_WINDOW_POS_FLAGS(embedding_flags(desktop.raised)),
        )
        .map_err(|error| PlayerError::Embedding(error.to_string()))?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn embed_player_window(
    _player_window: isize,
    _desktop: DesktopHost,
    _width: i32,
    _height: i32,
) -> Result<(), PlayerError> {
    Err(PlayerError::MissingDesktopHost)
}

fn maintain_window_order(
    player_window: isize,
    shell_view: isize,
) -> (Arc<AtomicBool>, JoinHandle<()>) {
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    let handle = thread::spawn(move || {
        while !thread_stop.load(Ordering::Acquire) {
            ensure_window_order(player_window, shell_view);
            thread::sleep(Duration::from_millis(250));
        }
    });
    (stop, handle)
}

#[cfg(windows)]
fn ensure_window_order(player_window: isize, shell_view: isize) {
    use core::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        IsWindow, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    };

    let player = HWND(player_window as *mut c_void);
    let shell = HWND(shell_view as *mut c_void);
    unsafe {
        if IsWindow(Some(player)).as_bool() && IsWindow(Some(shell)).as_bool() {
            let _ = SetWindowPos(
                player,
                Some(shell),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}

#[cfg(not(windows))]
fn ensure_window_order(_player_window: isize, _shell_view: isize) {}

#[cfg(test)]
mod tests {
    use super::embedding_flags;

    const SWP_NO_Z_ORDER: u32 = 0x0004;
    const SWP_FRAME_CHANGED: u32 = 0x0020;

    #[test]
    fn embedding_recreates_the_frame_and_preserves_the_expected_z_order() {
        let raised_flags = embedding_flags(true);
        let legacy_flags = embedding_flags(false);

        assert_eq!(raised_flags & SWP_FRAME_CHANGED, SWP_FRAME_CHANGED);
        assert_eq!(raised_flags & SWP_NO_Z_ORDER, 0);
        assert_eq!(legacy_flags & SWP_FRAME_CHANGED, SWP_FRAME_CHANGED);
        assert_eq!(legacy_flags & SWP_NO_Z_ORDER, SWP_NO_Z_ORDER);
    }
}
