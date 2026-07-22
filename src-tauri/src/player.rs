//! 启动并通过命名管道控制随应用分发的 mpv。

use crate::model::{AppSettings, DisplayMode, MediaKind};
use serde_json::json;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
#[cfg(windows)]
use std::os::windows::io::OwnedHandle;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use thiserror::Error;

/// 表示 Windows 虚拟桌面中的播放器目标矩形。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScreenRegion {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// 返回覆盖全部输入矩形的最小联合区域。
pub fn union_regions(regions: &[ScreenRegion]) -> Option<ScreenRegion> {
    regions.first()?;
    let left = regions.iter().map(|region| region.x).min()?;
    let top = regions.iter().map(|region| region.y).min()?;
    let right = regions
        .iter()
        .map(|region| region.x.saturating_add(region.width))
        .max()?;
    let bottom = regions
        .iter()
        .map(|region| region.y.saturating_add(region.height))
        .max()?;
    Some(ScreenRegion {
        x: left,
        y: top,
        width: right.saturating_sub(left),
        height: bottom.saturating_sub(top),
    })
}

/// 当最大播放位置差超过 100ms 时返回用于校准的最前位置。
pub fn drift_correction_target(positions: &[f64]) -> Option<f64> {
    let minimum = positions.iter().copied().reduce(f64::min)?;
    let maximum = positions.iter().copied().reduce(f64::max)?;
    (maximum - minimum > 0.1).then_some(maximum)
}

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
        format!(
            "--video-aspect-override={}",
            settings.aspect_ratio.mpv_value(screen_width, screen_height)
        ),
        format!("--scale={}", settings.anti_aliasing.mpv_scale()),
    ];
    if kind == MediaKind::Video && settings.frame_rate > 0 {
        arguments.push(format!("--vf=fps={}", settings.frame_rate));
    }
    arguments.extend(settings.scale_mode.mpv_arguments().map(str::to_owned));
    if kind == MediaKind::Image {
        arguments.push("--image-display-duration=inf".to_owned());
    }
    arguments.push(media_path.to_string_lossy().into_owned());
    arguments
}

/// 构造可通过 mpv IPC 实时应用的画面和声音设置命令。
pub fn build_live_settings_commands(
    settings: &AppSettings,
    screen_width: i32,
    screen_height: i32,
    kind: MediaKind,
) -> Vec<serde_json::Value> {
    let [keep_aspect, panscan] = settings.scale_mode.mpv_arguments();
    let panscan = panscan
        .rsplit('=')
        .next()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);
    vec![
        json!({ "command": ["set_property", "keepaspect", keep_aspect.ends_with("yes")] }),
        json!({ "command": ["set_property", "panscan", panscan] }),
        json!({
            "command": [
                "set_property",
                "video-aspect-override",
                settings.aspect_ratio.mpv_value(screen_width, screen_height)
            ]
        }),
        json!({ "command": ["set_property", "scale", settings.anti_aliasing.mpv_scale()] }),
        json!({
            "command": [
                "set_property",
                "vf",
                if kind == MediaKind::Image || settings.frame_rate == 0 {
                    String::new()
                } else {
                    format!("fps={}", settings.frame_rate)
                }
            ]
        }),
        json!({ "command": ["set_property", "mute", settings.default_muted] }),
        json!({ "command": ["set_property", "volume", settings.volume] }),
    ]
}

fn loadfile_command(media_path: &Path, request_id: i64) -> serde_json::Value {
    json!({
        "command": ["loadfile", media_path.to_string_lossy(), "replace"],
        "request_id": request_id
    })
}

fn ipc_request_id(command: &serde_json::Value) -> Result<i64, PlayerError> {
    command
        .get("request_id")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "mpv IPC 请求缺少 request_id",
            )
            .into()
        })
}

fn validate_loadfile_response(response: &serde_json::Value) -> Result<(), PlayerError> {
    let status = response
        .get("error")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("mpv IPC 响应缺少 error 字段");
    if status == "success" {
        Ok(())
    } else {
        Err(PlayerError::Command(status.to_owned()))
    }
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
    #[error("mpv 拒绝播放命令：{0}")]
    Command(String),
    #[error("{0}")]
    Rollback(String),
    #[error("无法启动或控制 mpv：{0}")]
    Io(#[from] std::io::Error),
}

#[derive(Default)]
pub struct MpvPlayer {
    child: Option<Child>,
    #[cfg(windows)]
    job: Option<OwnedHandle>,
    pipe_name: Option<String>,
    player_window: Option<isize>,
    shell_view: Option<isize>,
    screen_size: Option<(i32, i32)>,
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
        let (width, height) = primary_screen_size();
        self.play_region(
            binary,
            media_path,
            kind,
            settings,
            ScreenRegion {
                x: 0,
                y: 0,
                width,
                height,
            },
        )
    }

    /// 在指定虚拟桌面矩形内启动并嵌入新的 mpv 进程。
    pub fn play_region(
        &mut self,
        binary: &Path,
        media_path: &Path,
        kind: MediaKind,
        settings: &AppSettings,
        region: ScreenRegion,
    ) -> Result<(), PlayerError> {
        if !binary.is_file() {
            return Err(PlayerError::MissingBinary);
        }
        self.stop();
        let desktop = desktop_host_window().ok_or(PlayerError::MissingDesktopHost)?;
        let screen_width = region.width;
        let screen_height = region.height;
        let window_title = format!("wall-wallpaper-{}", uuid::Uuid::new_v4());
        let pipe_name = format!(r"\\.\pipe\wall-mpv-{}", uuid::Uuid::new_v4());
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
        #[cfg(windows)]
        let job = match assign_child_to_kill_job(&child) {
            Ok(job) => job,
            Err(problem) => {
                terminate_child(&mut child);
                return Err(problem);
            }
        };
        let player_window = match wait_for_player_window(&window_title, &mut child) {
            Ok(window) => window,
            Err(error) => {
                terminate_child(&mut child);
                return Err(error);
            }
        };
        if let Err(error) = embed_player_window(player_window, desktop, region) {
            terminate_child(&mut child);
            return Err(error);
        }
        self.child = Some(child);
        #[cfg(windows)]
        {
            self.job = Some(job);
        }
        self.pipe_name = Some(pipe_name);
        self.player_window = Some(player_window);
        self.shell_view = desktop.raised.then_some(desktop.shell_view);
        self.screen_size = Some((screen_width, screen_height));
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

    /// 将无需重启播放器的设置通过 IPC 应用到当前 mpv。
    pub fn apply_settings(
        &self,
        settings: &AppSettings,
        kind: MediaKind,
    ) -> Result<(), PlayerError> {
        let (width, height) = self
            .screen_size
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotConnected, "mpv 尚未运行"))?;
        for command in build_live_settings_commands(settings, width, height, kind) {
            self.send(command)?;
        }
        Ok(())
    }

    fn load_media(&self, media_path: &Path) -> Result<(), PlayerError> {
        let response = self.request(loadfile_command(media_path, 2))?;
        validate_loadfile_response(&response)
    }

    /// 停止当前 mpv 进程；管道不可用时直接终止子进程。
    pub fn stop(&mut self) {
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
        #[cfg(windows)]
        {
            self.job = None;
        }
        self.pipe_name = None;
        self.player_window = None;
        self.shell_view = None;
        self.screen_size = None;
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

    fn time_position(&self) -> Result<Option<f64>, PlayerError> {
        let response = self.request(json!({
            "command": ["get_property", "time-pos"],
            "request_id": 1
        }))?;
        Ok(response.get("data").and_then(serde_json::Value::as_f64))
    }

    fn has_exited(&mut self) -> Result<bool, PlayerError> {
        match self.child.as_mut() {
            Some(child) => Ok(child.try_wait()?.is_some()),
            None => Ok(true),
        }
    }

    fn request(&self, command: serde_json::Value) -> Result<serde_json::Value, PlayerError> {
        let request_id = ipc_request_id(&command)?;
        let pipe_name = self
            .pipe_name
            .as_ref()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotConnected, "mpv 尚未运行"))?;
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut pipe = loop {
            match OpenOptions::new().read(true).write(true).open(pipe_name) {
                Ok(pipe) => break pipe,
                Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(40)),
                Err(error) => return Err(error.into()),
            }
        };
        serde_json::to_writer(&mut pipe, &command).map_err(std::io::Error::other)?;
        pipe.write_all(b"\n")?;
        read_ipc_response(
            &mut pipe,
            request_id,
            Instant::now() + Duration::from_secs(2),
        )
    }
}

#[cfg(windows)]
fn read_ipc_response(
    pipe: &mut File,
    request_id: i64,
    deadline: Instant,
) -> Result<serde_json::Value, PlayerError> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Pipes::PeekNamedPipe;

    let handle = HANDLE(pipe.as_raw_handle());
    let mut pending = Vec::new();
    loop {
        if Instant::now() >= deadline {
            return Err(
                std::io::Error::new(std::io::ErrorKind::TimedOut, "mpv IPC 响应超时").into(),
            );
        }
        let mut available = 0;
        unsafe { PeekNamedPipe(handle, None, 0, None, Some(&mut available), None) }
            .map_err(|problem| std::io::Error::other(problem.to_string()))?;
        if available == 0 {
            thread::sleep(Duration::from_millis(10));
            continue;
        }
        let start = pending.len();
        pending.resize(start + available as usize, 0);
        pipe.read_exact(&mut pending[start..])?;
        while let Some(end) = pending.iter().position(|byte| *byte == b'\n') {
            let line = pending.drain(..=end).collect::<Vec<_>>();
            let value: serde_json::Value =
                serde_json::from_slice(&line).map_err(std::io::Error::other)?;
            if value.get("request_id").and_then(serde_json::Value::as_i64) == Some(request_id) {
                return Ok(value);
            }
        }
    }
}

#[cfg(not(windows))]
fn read_ipc_response(
    pipe: &mut File,
    request_id: i64,
    _deadline: Instant,
) -> Result<serde_json::Value, PlayerError> {
    use std::io::{BufRead, BufReader};

    let mut reader = BufReader::new(pipe);
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "mpv IPC 未返回响应",
            )
            .into());
        }
        let value: serde_json::Value =
            serde_json::from_str(&line).map_err(std::io::Error::other)?;
        if value.get("request_id").and_then(serde_json::Value::as_i64) == Some(request_id) {
            return Ok(value);
        }
    }
}

impl Drop for MpvPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

struct PlayerGroup {
    media_id: String,
    binary: PathBuf,
    media_path: PathBuf,
    kind: MediaKind,
    settings: AppSettings,
    mode: DisplayMode,
    display_ids: Vec<String>,
    regions: Vec<ScreenRegion>,
    paused: bool,
    muted: bool,
    volume: u8,
    players: Vec<MpvPlayer>,
}

#[allow(clippy::too_many_arguments)]
fn can_hot_switch(
    group: &PlayerGroup,
    binary: &Path,
    kind: MediaKind,
    settings: &AppSettings,
    mode: DisplayMode,
    display_ids: &[String],
    regions: &[ScreenRegion],
    player_count: usize,
) -> bool {
    group.binary == binary
        && group.kind == kind
        && group.settings.hardware_decoding == settings.hardware_decoding
        && group.mode == mode
        && group.display_ids == display_ids
        && group.regions == regions
        && group.players.len() == player_count
}

#[derive(Debug, Eq, PartialEq)]
enum FailedSwitchDisposition {
    RestartKeep,
    RestartDrop,
    RejectKeep,
    RejectDrop(String),
}

fn failed_switch_disposition(
    problem: &PlayerError,
    rollback_errors: &[String],
) -> FailedSwitchDisposition {
    let restart = matches!(problem, PlayerError::Io(_) | PlayerError::EarlyExit(_));
    match (restart, rollback_errors.is_empty()) {
        (true, true) => FailedSwitchDisposition::RestartKeep,
        (true, false) => FailedSwitchDisposition::RestartDrop,
        (false, true) => FailedSwitchDisposition::RejectKeep,
        (false, false) => FailedSwitchDisposition::RejectDrop(format!(
            "{problem}；回滚失败：{}",
            rollback_errors.join("；")
        )),
    }
}

/// 管理独立显示器和联动显示器组中的全部 mpv 实例。
#[derive(Default)]
pub struct MpvPlayerManager {
    groups: HashMap<String, PlayerGroup>,
    order_stop: Option<Arc<AtomicBool>>,
    order_thread: Option<JoinHandle<()>>,
}

impl MpvPlayerManager {
    /// 判断指定显示目标当前是否有受管播放器组。
    pub fn has_target(&self, target_id: &str) -> bool {
        self.groups.contains_key(target_id)
    }

    fn stop_window_order(&mut self) {
        if let Some(stop) = self.order_stop.take() {
            stop.store(true, Ordering::Release);
        }
        if let Some(thread) = self.order_thread.take() {
            let _ = thread.join();
        }
    }

    fn refresh_window_order(&mut self) {
        self.stop_window_order();
        let targets = self
            .groups
            .values()
            .flat_map(|group| &group.players)
            .filter_map(|player| Some((player.player_window?, player.shell_view?)))
            .collect::<Vec<_>>();
        if !targets.is_empty() {
            let (stop, thread) = maintain_window_order(targets);
            self.order_stop = Some(stop);
            self.order_thread = Some(thread);
        }
    }

    /// 兼容单显示器调用，并停止此前全部播放器。
    pub fn play(
        &mut self,
        media_id: &str,
        binary: &Path,
        media_path: &Path,
        kind: MediaKind,
        settings: &AppSettings,
    ) -> Result<(), PlayerError> {
        self.stop();
        let (width, height) = primary_screen_size();
        self.play_target(
            "display:primary",
            media_id,
            binary,
            media_path,
            kind,
            settings,
            DisplayMode::Independent,
            &["primary".to_owned()],
            &[ScreenRegion {
                x: 0,
                y: 0,
                width,
                height,
            }],
        )
    }

    /// 在一个独立目标、复制组或铺展组中播放指定媒体。
    #[allow(clippy::too_many_arguments)]
    pub fn play_target(
        &mut self,
        target_id: &str,
        media_id: &str,
        binary: &Path,
        media_path: &Path,
        kind: MediaKind,
        settings: &AppSettings,
        mode: DisplayMode,
        display_ids: &[String],
        regions: &[ScreenRegion],
    ) -> Result<(), PlayerError> {
        self.play_target_configured(
            target_id,
            media_id,
            binary,
            media_path,
            kind,
            settings,
            mode,
            display_ids,
            regions,
            false,
            settings.default_muted,
            settings.volume,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn try_hot_switch(
        &mut self,
        target_id: &str,
        media_id: &str,
        binary: &Path,
        media_path: &Path,
        kind: MediaKind,
        settings: &AppSettings,
        mode: DisplayMode,
        display_ids: &[String],
        regions: &[ScreenRegion],
        target_regions: &[ScreenRegion],
        paused: bool,
        muted: bool,
        volume: u8,
    ) -> Result<bool, PlayerError> {
        let Some(mut group) = self.groups.remove(target_id) else {
            return Ok(false);
        };
        if !can_hot_switch(
            &group,
            binary,
            kind,
            settings,
            mode,
            display_ids,
            regions,
            target_regions.len(),
        ) {
            self.groups.insert(target_id.to_owned(), group);
            return Ok(false);
        }
        for player in &mut group.players {
            if !matches!(player.has_exited(), Ok(false)) {
                self.groups.insert(target_id.to_owned(), group);
                return Ok(false);
            }
        }

        let previous_path = group.media_path.clone();
        let previous_settings = group.settings.clone();
        let previous_paused = group.paused;
        let previous_muted = group.muted;
        let previous_volume = group.volume;
        let switch_result = (|| {
            for (index, player) in group.players.iter().enumerate() {
                player.load_media(media_path)?;
                let mut player_settings = settings.clone();
                if mode == DisplayMode::Clone && index > 0 {
                    player_settings.default_muted = true;
                }
                player.apply_settings(&player_settings, kind)?;
            }
            if mode == DisplayMode::Clone && group.players.len() > 1 {
                for player in &group.players {
                    player.set_paused(true)?;
                    player.send(json!({ "command": ["set_property", "time-pos", 0] }))?;
                }
                for player in &group.players {
                    player.set_paused(false)?;
                }
            }
            for (index, player) in group.players.iter().enumerate() {
                player.set_paused(paused)?;
                player.set_muted(muted || index > 0)?;
                player.set_volume(volume)?;
            }
            Ok(())
        })();
        if let Err(problem) = switch_result {
            let mut rollback_errors = Vec::new();
            for (index, player) in group.players.iter().enumerate() {
                if let Err(rollback) = player.load_media(&previous_path) {
                    rollback_errors.push(format!(
                        "实例 {} 恢复媒体失败：{rollback}",
                        index.saturating_add(1)
                    ));
                }
                let mut player_settings = previous_settings.clone();
                if group.mode == DisplayMode::Clone && index > 0 {
                    player_settings.default_muted = true;
                }
                if let Err(rollback) = player.apply_settings(&player_settings, group.kind) {
                    rollback_errors.push(format!(
                        "实例 {} 恢复设置失败：{rollback}",
                        index.saturating_add(1)
                    ));
                }
                if let Err(rollback) = player.set_paused(previous_paused) {
                    rollback_errors.push(format!(
                        "实例 {} 恢复暂停状态失败：{rollback}",
                        index.saturating_add(1)
                    ));
                }
                if let Err(rollback) = player.set_muted(previous_muted || index > 0) {
                    rollback_errors.push(format!(
                        "实例 {} 恢复静音状态失败：{rollback}",
                        index.saturating_add(1)
                    ));
                }
                if let Err(rollback) = player.set_volume(previous_volume) {
                    rollback_errors.push(format!(
                        "实例 {} 恢复音量失败：{rollback}",
                        index.saturating_add(1)
                    ));
                }
            }
            return match failed_switch_disposition(&problem, &rollback_errors) {
                FailedSwitchDisposition::RestartKeep => {
                    self.groups.insert(target_id.to_owned(), group);
                    Ok(false)
                }
                FailedSwitchDisposition::RestartDrop => {
                    for player in &mut group.players {
                        player.stop();
                    }
                    Ok(false)
                }
                FailedSwitchDisposition::RejectKeep => {
                    self.groups.insert(target_id.to_owned(), group);
                    Err(problem)
                }
                FailedSwitchDisposition::RejectDrop(message) => {
                    for player in &mut group.players {
                        player.stop();
                    }
                    Err(PlayerError::Rollback(message))
                }
            };
        }

        group.media_id = media_id.to_owned();
        group.binary = binary.to_path_buf();
        group.media_path = media_path.to_path_buf();
        group.kind = kind;
        group.settings = settings.clone();
        group.mode = mode;
        group.display_ids = display_ids.to_vec();
        group.regions = regions.to_vec();
        group.paused = paused;
        group.muted = muted;
        group.volume = volume;
        self.groups.insert(target_id.to_owned(), group);
        Ok(true)
    }

    /// 完成初始暂停与声音配置后，再原子替换占用相同显示器的旧播放器组。
    #[allow(clippy::too_many_arguments)]
    pub fn play_target_configured(
        &mut self,
        target_id: &str,
        media_id: &str,
        binary: &Path,
        media_path: &Path,
        kind: MediaKind,
        settings: &AppSettings,
        mode: DisplayMode,
        display_ids: &[String],
        regions: &[ScreenRegion],
        paused: bool,
        muted: bool,
        volume: u8,
    ) -> Result<(), PlayerError> {
        let target_regions = match mode {
            DisplayMode::Span => {
                vec![union_regions(regions).ok_or(PlayerError::MissingDesktopHost)?]
            }
            DisplayMode::Independent => {
                vec![*regions.first().ok_or(PlayerError::MissingDesktopHost)?]
            }
            DisplayMode::Clone => regions.to_vec(),
        };
        if self.try_hot_switch(
            target_id,
            media_id,
            binary,
            media_path,
            kind,
            settings,
            mode,
            display_ids,
            regions,
            &target_regions,
            paused,
            muted,
            volume,
        )? {
            return Ok(());
        }
        let mut players: Vec<MpvPlayer> = Vec::with_capacity(target_regions.len());
        for (index, region) in target_regions.iter().copied().enumerate() {
            let mut player = MpvPlayer::default();
            let mut player_settings = settings.clone();
            if mode == DisplayMode::Clone && index > 0 {
                player_settings.default_muted = true;
            }
            if let Err(error) =
                player.play_region(binary, media_path, kind, &player_settings, region)
            {
                for started in &mut players {
                    started.stop();
                }
                return Err(error);
            }
            players.push(player);
        }
        if mode == DisplayMode::Clone && players.len() > 1 {
            for player in &players {
                player.set_paused(true)?;
                player.send(json!({ "command": ["set_property", "time-pos", 0] }))?;
            }
            for player in &players {
                player.set_paused(false)?;
            }
        }
        for (index, player) in players.iter().enumerate() {
            player.set_paused(paused)?;
            player.set_muted(muted || index > 0)?;
            player.set_volume(volume)?;
        }
        let replaced = self
            .groups
            .iter()
            .filter(|(existing_id, group)| {
                existing_id.as_str() == target_id
                    || group
                        .display_ids
                        .iter()
                        .any(|display_id| display_ids.contains(display_id))
            })
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        if !replaced.is_empty() {
            self.stop_window_order();
        }
        for key in replaced {
            if let Some(mut group) = self.groups.remove(&key) {
                for player in &mut group.players {
                    player.stop();
                }
            }
        }
        self.groups.insert(
            target_id.to_owned(),
            PlayerGroup {
                media_id: media_id.to_owned(),
                binary: binary.to_path_buf(),
                media_path: media_path.to_path_buf(),
                kind,
                settings: settings.clone(),
                mode,
                display_ids: display_ids.to_vec(),
                regions: regions.to_vec(),
                paused,
                muted,
                volume,
                players,
            },
        );
        self.refresh_window_order();
        Ok(())
    }

    /// 暂停或继续全部显示目标。
    pub fn set_paused(&mut self, paused: bool) -> Result<(), PlayerError> {
        for group in self.groups.values_mut() {
            for player in &group.players {
                player.set_paused(paused)?;
            }
            group.paused = paused;
        }
        Ok(())
    }

    /// 暂停或继续一个显示目标。
    pub fn set_target_paused(&mut self, target_id: &str, paused: bool) -> Result<(), PlayerError> {
        let group = self
            .groups
            .get_mut(target_id)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "显示目标不存在"))?;
        for player in &group.players {
            player.set_paused(paused)?;
        }
        group.paused = paused;
        Ok(())
    }

    /// 设置每个显示目标的单一音源静音状态。
    pub fn set_muted(&mut self, muted: bool) -> Result<(), PlayerError> {
        for group in self.groups.values_mut() {
            for (index, player) in group.players.iter().enumerate() {
                player.set_muted(muted || index > 0)?;
            }
            group.muted = muted;
        }
        Ok(())
    }

    /// 设置一个显示目标的单一音源静音状态。
    pub fn set_target_muted(&mut self, target_id: &str, muted: bool) -> Result<(), PlayerError> {
        let group = self
            .groups
            .get_mut(target_id)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "显示目标不存在"))?;
        for (index, player) in group.players.iter().enumerate() {
            player.set_muted(muted || index > 0)?;
        }
        group.muted = muted;
        Ok(())
    }

    /// 设置一个显示目标的音量。
    pub fn set_target_volume(&mut self, target_id: &str, volume: u8) -> Result<(), PlayerError> {
        let group = self
            .groups
            .get_mut(target_id)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "显示目标不存在"))?;
        for player in &group.players {
            player.set_volume(volume)?;
        }
        group.volume = volume;
        Ok(())
    }

    /// 设置全部显示目标的音量。
    pub fn set_volume(&mut self, volume: u8) -> Result<(), PlayerError> {
        for group in self.groups.values_mut() {
            for player in &group.players {
                player.set_volume(volume)?;
            }
            group.volume = volume;
        }
        Ok(())
    }

    /// 动态更新全部显示目标的缩放方式。
    pub fn set_scale_mode(&mut self, settings: &AppSettings) -> Result<(), PlayerError> {
        for group in self.groups.values_mut() {
            for player in &group.players {
                player.set_scale_mode(settings)?;
            }
            group.settings.scale_mode = settings.scale_mode;
        }
        Ok(())
    }

    /// 动态更新全部无需重启的播放器设置。
    pub fn apply_settings(&mut self, settings: &AppSettings) -> Result<(), PlayerError> {
        for group in self.groups.values_mut() {
            for (index, player) in group.players.iter().enumerate() {
                let mut player_settings = settings.clone();
                if group.mode == DisplayMode::Clone && index > 0 {
                    player_settings.default_muted = true;
                }
                player.apply_settings(&player_settings, group.kind)?;
            }
            group.settings = settings.clone();
        }
        Ok(())
    }

    /// 更新使用指定壁纸的显示目标；硬件解码变化时仅重启相关播放器组。
    pub fn apply_media_settings(
        &mut self,
        media_id: &str,
        settings: &AppSettings,
    ) -> Result<(), PlayerError> {
        let keys = self
            .groups
            .iter()
            .filter(|(_, group)| group.media_id == media_id)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in keys {
            let Some(mut group) = self.groups.remove(&key) else {
                continue;
            };
            if group.settings.hardware_decoding != settings.hardware_decoding {
                let restart = self.play_target(
                    &key,
                    media_id,
                    &group.binary,
                    &group.media_path,
                    group.kind,
                    settings,
                    group.mode,
                    &group.display_ids,
                    &group.regions,
                );
                if let Err(problem) = restart {
                    self.groups.insert(key, group);
                    return Err(problem);
                }
                for player in &mut group.players {
                    player.stop();
                }
            } else {
                let previous_settings = group.settings.clone();
                for (index, player) in group.players.iter().enumerate() {
                    let mut player_settings = settings.clone();
                    if group.mode == DisplayMode::Clone && index > 0 {
                        player_settings.default_muted = true;
                    }
                    if let Err(problem) = player.apply_settings(&player_settings, group.kind) {
                        for (rollback_index, rollback_player) in group.players.iter().enumerate() {
                            let mut rollback_settings = previous_settings.clone();
                            if group.mode == DisplayMode::Clone && rollback_index > 0 {
                                rollback_settings.default_muted = true;
                            }
                            let _ = rollback_player.apply_settings(&rollback_settings, group.kind);
                        }
                        self.groups.insert(key, group);
                        return Err(problem);
                    }
                }
                group.settings = settings.clone();
                self.groups.insert(key, group);
            }
        }
        Ok(())
    }

    /// 校准复制组中超过 100ms 的播放漂移。
    pub fn sync_clone_groups(&self) -> Result<(), PlayerError> {
        for group in self
            .groups
            .values()
            .filter(|group| group.mode == DisplayMode::Clone && group.players.len() > 1)
        {
            let positions = group
                .players
                .iter()
                .map(MpvPlayer::time_position)
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            if let Some(target) = drift_correction_target(&positions) {
                for player in &group.players {
                    player.send(json!({ "command": ["set_property", "time-pos", target] }))?;
                }
            }
        }
        Ok(())
    }

    /// 移除任一 mpv 子进程已经退出的显示目标并返回其标识。
    pub fn take_exited_targets(&mut self) -> Result<Vec<String>, PlayerError> {
        let mut exited = Vec::new();
        for (target_id, group) in &mut self.groups {
            let mut group_exited = false;
            for player in &mut group.players {
                group_exited |= player.has_exited()?;
            }
            if group_exited {
                exited.push(target_id.clone());
            }
        }
        for target_id in &exited {
            self.stop_target(target_id);
        }
        Ok(exited)
    }

    /// 停止使用指定壁纸的全部显示目标。
    pub fn stop_media(&mut self, media_id: &str) {
        let keys = self
            .groups
            .iter()
            .filter(|(_, group)| group.media_id == media_id)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        if !keys.is_empty() {
            self.stop_window_order();
        }
        for key in keys {
            if let Some(mut group) = self.groups.remove(&key) {
                for player in &mut group.players {
                    player.stop();
                }
            }
        }
        self.refresh_window_order();
    }

    /// 停止并移除一个显示目标。
    pub fn stop_target(&mut self, target_id: &str) {
        if self.groups.contains_key(target_id) {
            self.stop_window_order();
        }
        if let Some(mut group) = self.groups.remove(target_id) {
            for player in &mut group.players {
                player.stop();
            }
        }
        self.refresh_window_order();
    }

    /// 停止并清空全部显示目标。
    pub fn stop(&mut self) {
        self.stop_window_order();
        for group in self.groups.values_mut() {
            for player in &mut group.players {
                player.stop();
            }
        }
        self.groups.clear();
    }
}

impl Drop for MpvPlayerManager {
    fn drop(&mut self) {
        self.stop();
    }
}

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(windows)]
fn assign_child_to_kill_job(child: &Child) -> Result<OwnedHandle, PlayerError> {
    use core::ffi::c_void;
    use std::mem::size_of;
    use std::os::windows::io::{AsRawHandle as _, FromRawHandle as _};
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };
    use windows::core::PCWSTR;

    let raw_job = unsafe { CreateJobObjectW(None, PCWSTR::null()) }
        .map_err(|problem| std::io::Error::other(problem.to_string()))?;
    let job = unsafe { OwnedHandle::from_raw_handle(raw_job.0) };
    let job_handle = HANDLE(job.as_raw_handle());
    let mut information = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    information.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

    unsafe {
        SetInformationJobObject(
            job_handle,
            JobObjectExtendedLimitInformation,
            &information as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION as *const c_void,
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
        .map_err(|problem| std::io::Error::other(problem.to_string()))?;
        AssignProcessToJobObject(job_handle, HANDLE(child.as_raw_handle()))
            .map_err(|problem| std::io::Error::other(problem.to_string()))?;
    }
    Ok(job)
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
    region: ScreenRegion,
) -> Result<(), PlayerError> {
    use core::ffi::c_void;
    use windows::Win32::Foundation::{COLORREF, HWND, POINT};
    use windows::Win32::Graphics::Gdi::ScreenToClient;
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
        let mut origin = POINT {
            x: region.x,
            y: region.y,
        };
        ScreenToClient(host, &mut origin)
            .ok()
            .map_err(|error| PlayerError::Embedding(error.to_string()))?;
        let insert_after = desktop.raised.then_some(shell_view);
        SetWindowPos(
            player,
            insert_after,
            origin.x,
            origin.y,
            region.width,
            region.height,
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
    _region: ScreenRegion,
) -> Result<(), PlayerError> {
    Err(PlayerError::MissingDesktopHost)
}

fn window_order_chain(mut targets: Vec<(isize, isize)>) -> Vec<(isize, isize)> {
    targets.sort_unstable_by_key(|(player, shell)| (*shell, *player));
    let mut previous = None;
    targets
        .into_iter()
        .map(|(player, shell)| {
            let insert_after = match previous {
                Some((previous_shell, previous_player)) if previous_shell == shell => {
                    previous_player
                }
                _ => shell,
            };
            previous = Some((shell, player));
            (player, insert_after)
        })
        .collect()
}

fn maintain_window_order(targets: Vec<(isize, isize)>) -> (Arc<AtomicBool>, JoinHandle<()>) {
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    let chain = window_order_chain(targets);
    let handle = thread::spawn(move || {
        while !thread_stop.load(Ordering::Acquire) {
            for &(player_window, insert_after) in &chain {
                ensure_window_order(player_window, insert_after);
            }
            thread::sleep(Duration::from_millis(250));
        }
    });
    (stop, handle)
}

#[cfg(windows)]
fn ensure_window_order(player_window: isize, insert_after: isize) {
    use core::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        IsWindow, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    };

    let player = HWND(player_window as *mut c_void);
    let anchor = HWND(insert_after as *mut c_void);
    unsafe {
        if IsWindow(Some(player)).as_bool() && IsWindow(Some(anchor)).as_bool() {
            let _ = SetWindowPos(
                player,
                Some(anchor),
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
fn ensure_window_order(_player_window: isize, _insert_after: isize) {}

#[cfg(test)]
mod tests {
    use super::{
        FailedSwitchDisposition, MpvPlayer, PlayerError, PlayerGroup, ScreenRegion, can_hot_switch,
        embedding_flags, failed_switch_disposition, ipc_request_id, loadfile_command,
        validate_loadfile_response, window_order_chain,
    };
    #[cfg(windows)]
    use super::{assign_child_to_kill_job, terminate_child};
    use crate::model::{AppSettings, DisplayMode, MediaKind};
    use serde_json::json;
    use std::path::{Path, PathBuf};
    #[cfg(windows)]
    use std::{
        thread,
        time::{Duration, Instant},
    };

    const SWP_NO_Z_ORDER: u32 = 0x0004;
    const SWP_FRAME_CHANGED: u32 = 0x0020;

    #[cfg(windows)]
    #[test]
    fn dropping_job_terminates_assigned_child() {
        use std::os::windows::process::CommandExt as _;
        use std::process::Command;

        let mut command = Command::new("powershell.exe");
        command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-Command",
                "Start-Sleep -Seconds 30",
            ])
            .creation_flags(0x08000000);
        let mut child = command.spawn().expect("测试子进程应能启动");
        let job = assign_child_to_kill_job(&child).expect("子进程应能加入 Job Object");

        drop(job);
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if child.try_wait().expect("应能读取测试子进程状态").is_some() {
                return;
            }
            thread::sleep(Duration::from_millis(25));
        }

        terminate_child(&mut child);
        panic!("Job Object 关闭后测试子进程仍在运行");
    }

    #[test]
    fn embedding_recreates_the_frame_and_preserves_the_expected_z_order() {
        let raised_flags = embedding_flags(true);
        let legacy_flags = embedding_flags(false);

        assert_eq!(raised_flags & SWP_FRAME_CHANGED, SWP_FRAME_CHANGED);
        assert_eq!(raised_flags & SWP_NO_Z_ORDER, 0);
        assert_eq!(legacy_flags & SWP_FRAME_CHANGED, SWP_FRAME_CHANGED);
        assert_eq!(legacy_flags & SWP_NO_Z_ORDER, SWP_NO_Z_ORDER);
    }

    #[test]
    fn multiple_windows_form_one_stable_desktop_chain() {
        assert_eq!(
            window_order_chain(vec![(30, 7), (20, 7)]),
            vec![(20, 7), (30, 20)]
        );
    }

    #[test]
    fn hot_switch_requires_the_same_player_contract() {
        let region = ScreenRegion {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let mut group = PlayerGroup {
            media_id: "old".to_owned(),
            binary: PathBuf::from("mpv.exe"),
            media_path: PathBuf::from("old.mp4"),
            kind: MediaKind::Video,
            settings: AppSettings::default(),
            mode: DisplayMode::Independent,
            display_ids: vec!["DISPLAY1".to_owned()],
            regions: vec![region],
            paused: false,
            muted: true,
            volume: 0,
            players: vec![MpvPlayer::default()],
        };

        assert!(can_hot_switch(
            &group,
            Path::new("mpv.exe"),
            MediaKind::Video,
            &AppSettings::default(),
            DisplayMode::Independent,
            &["DISPLAY1".to_owned()],
            &[region],
            1,
        ));
        assert!(!can_hot_switch(
            &group,
            Path::new("mpv.exe"),
            MediaKind::Image,
            &AppSettings::default(),
            DisplayMode::Independent,
            &["DISPLAY1".to_owned()],
            &[region],
            1,
        ));
        assert!(!can_hot_switch(
            &group,
            Path::new("other-mpv.exe"),
            MediaKind::Video,
            &AppSettings::default(),
            DisplayMode::Independent,
            &["DISPLAY1".to_owned()],
            &[region],
            1,
        ));
        assert!(!can_hot_switch(
            &group,
            Path::new("mpv.exe"),
            MediaKind::Video,
            &AppSettings::default(),
            DisplayMode::Independent,
            &["DISPLAY1".to_owned()],
            &[region],
            2,
        ));
        assert!(!can_hot_switch(
            &group,
            Path::new("mpv.exe"),
            MediaKind::Video,
            &AppSettings {
                hardware_decoding: false,
                ..Default::default()
            },
            DisplayMode::Independent,
            &["DISPLAY1".to_owned()],
            &[region],
            1,
        ));
        assert!(!can_hot_switch(
            &group,
            Path::new("mpv.exe"),
            MediaKind::Video,
            &AppSettings::default(),
            DisplayMode::Clone,
            &["DISPLAY1".to_owned()],
            &[region],
            1,
        ));
        assert!(!can_hot_switch(
            &group,
            Path::new("mpv.exe"),
            MediaKind::Video,
            &AppSettings::default(),
            DisplayMode::Independent,
            &["DISPLAY2".to_owned()],
            &[region],
            1,
        ));
        assert!(!can_hot_switch(
            &group,
            Path::new("mpv.exe"),
            MediaKind::Video,
            &AppSettings::default(),
            DisplayMode::Independent,
            &["DISPLAY1".to_owned()],
            &[ScreenRegion { x: 10, ..region }],
            1,
        ));

        group.kind = MediaKind::Image;
        assert!(can_hot_switch(
            &group,
            Path::new("mpv.exe"),
            MediaKind::Image,
            &AppSettings::default(),
            DisplayMode::Independent,
            &["DISPLAY1".to_owned()],
            &[region],
            1,
        ));
    }

    #[test]
    fn hot_switch_loadfile_command_replaces_current_media() {
        let command = loadfile_command(Path::new(r"D:\Wallpapers\next.mp4"), 2);
        assert_eq!(
            command,
            json!({
                "command": ["loadfile", r"D:\Wallpapers\next.mp4", "replace"],
                "request_id": 2
            })
        );
        assert_eq!(ipc_request_id(&command).expect("request ID"), 2);
    }

    #[test]
    fn loadfile_response_rejects_mpv_errors() {
        assert!(validate_loadfile_response(&json!({ "error": "success" })).is_ok());

        let error = validate_loadfile_response(&json!({ "error": "loading failed" }))
            .expect_err("mpv rejection must be returned");
        assert!(matches!(
            error,
            PlayerError::Command(message) if message == "loading failed"
        ));
    }

    #[test]
    fn failed_switch_drops_a_group_when_rollback_is_incomplete() {
        let command_error = PlayerError::Command("loading failed".to_owned());
        assert_eq!(
            failed_switch_disposition(&command_error, &[]),
            FailedSwitchDisposition::RejectKeep
        );
        assert_eq!(
            failed_switch_disposition(&command_error, &["实例 1 恢复媒体失败".to_owned()]),
            FailedSwitchDisposition::RejectDrop(
                "mpv 拒绝播放命令：loading failed；回滚失败：实例 1 恢复媒体失败".to_owned()
            )
        );

        let io_error = PlayerError::Io(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "pipe closed",
        ));
        assert_eq!(
            failed_switch_disposition(&io_error, &[]),
            FailedSwitchDisposition::RestartKeep
        );
        assert_eq!(
            failed_switch_disposition(&io_error, &["实例 2 恢复设置失败".to_owned()]),
            FailedSwitchDisposition::RestartDrop
        );
    }
}
