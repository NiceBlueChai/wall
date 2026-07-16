//! 启动并通过命名管道控制随应用分发的 mpv。

use crate::model::{AppSettings, MediaKind};
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

/// 构造不读取用户配置、无交互并嵌入桌面宿主的 mpv 参数。
pub fn build_mpv_arguments(
    media_path: &Path,
    kind: MediaKind,
    parent_window: isize,
    pipe_name: &str,
    settings: &AppSettings,
) -> Vec<String> {
    let mut arguments = vec![
        "--no-config".to_owned(),
        "--terminal=no".to_owned(),
        "--force-window=yes".to_owned(),
        "--idle=no".to_owned(),
        "--loop-file=inf".to_owned(),
        "--no-osc".to_owned(),
        "--no-input-default-bindings".to_owned(),
        "--input-cursor=no".to_owned(),
        "--input-vo-keyboard=no".to_owned(),
        "--background-color=#000000".to_owned(),
        format!("--wid={parent_window}"),
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
    #[error("无法启动或控制 mpv：{0}")]
    Io(#[from] std::io::Error),
}

#[derive(Default)]
pub struct MpvPlayer {
    child: Option<Child>,
    pipe_name: Option<String>,
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
        let parent = desktop_host_window().ok_or(PlayerError::MissingDesktopHost)?;
        let pipe_name = format!(r"\\.\pipe\wall-mpv-{}", std::process::id());
        let arguments = build_mpv_arguments(media_path, kind, parent, &pipe_name, settings);
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
        self.child = Some(command.spawn()?);
        self.pipe_name = Some(pipe_name);
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
        if self.child.is_none() {
            return;
        }
        let _ = self.send(json!({ "command": ["quit"] }));
        if let Some(child) = self.child.as_mut() {
            thread::sleep(Duration::from_millis(80));
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        self.child = None;
        self.pipe_name = None;
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

#[cfg(windows)]
fn desktop_host_window() -> Option<isize> {
    use windows::Win32::Foundation::{HWND, LPARAM, TRUE, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, FindWindowExW, FindWindowW, SMTO_NORMAL, SendMessageTimeoutW,
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
            LPARAM(0),
            SMTO_NORMAL,
            1000,
            None,
        );
        let mut worker = HWND::default();
        let _ = EnumWindows(
            Some(find_worker),
            LPARAM((&mut worker as *mut HWND) as isize),
        );
        let host = if worker.is_invalid() { progman } else { worker };
        Some(host.0 as isize)
    }
}

#[cfg(not(windows))]
fn desktop_host_window() -> Option<isize> {
    None
}
