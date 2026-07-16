//! 将 Vue 和托盘操作桥接到同一套 WallCore、持久化与 mpv 实例。

use crate::core::WallCore;
use crate::model::{AppError, AppSettings, AppSnapshot, PauseReason, ScaleMode, WallpaperItem};
use crate::player::{MpvPlayer, PlayerError};
use crate::storage::Storage;
use crate::tray::TrayMenuState;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_opener::OpenerExt as _;

pub struct RuntimeState {
    core: Mutex<WallCore>,
    player: Mutex<MpvPlayer>,
    storage: Storage,
    mpv_binary: PathBuf,
    data_dir: PathBuf,
}

impl RuntimeState {
    /// 恢复本地状态并定位便携目录中的 mpv。
    pub fn load(app: &tauri::App) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = app.path().app_config_dir()?;
        let storage = Storage::new(data_dir.clone());
        let mut snapshot = storage.load()?;
        for item in &mut snapshot.library {
            item.missing = !Path::new(&item.path).is_file();
            if !item.missing {
                let _ = app.asset_protocol_scope().allow_file(&item.path);
            }
        }
        Ok(Self {
            core: Mutex::new(WallCore::new(snapshot)),
            player: Mutex::new(MpvPlayer::default()),
            storage,
            mpv_binary: resolve_mpv_binary(),
            data_dir,
        })
    }

    /// 将无法向界面返回的后台错误追加到本地日志。
    pub fn log_error(&self, context: &str, message: &str) {
        let logs = self.data_dir.join("logs");
        if std::fs::create_dir_all(&logs).is_err() {
            return;
        }
        let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(logs.join("wall.log"))
        else {
            return;
        };
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or_default();
        let _ = writeln!(file, "{timestamp} [{context}] {message}");
    }

    fn core(&self) -> Result<MutexGuard<'_, WallCore>, AppError> {
        self.core
            .lock()
            .map_err(|_| error("state_poisoned", "应用状态暂时不可用", false))
    }

    fn player(&self) -> Result<MutexGuard<'_, MpvPlayer>, AppError> {
        self.player
            .lock()
            .map_err(|_| error("player_poisoned", "播放器状态暂时不可用", true))
    }

    fn commit(&self, app: &AppHandle, core: &WallCore) -> Result<AppSnapshot, AppError> {
        let snapshot = core.snapshot().clone();
        self.storage
            .save(&snapshot)
            .map_err(|problem| error("storage_failed", &problem.to_string(), true))?;
        app.emit("app-state://changed", &snapshot)
            .map_err(|problem| error("event_failed", &problem.to_string(), true))?;
        if let Some(tray) = app.try_state::<TrayMenuState>() {
            tray.update(&snapshot);
        }
        Ok(snapshot)
    }
}

#[tauri::command]
pub fn bootstrap(state: State<'_, RuntimeState>) -> Result<AppSnapshot, AppError> {
    Ok(state.core()?.snapshot().clone())
}

#[tauri::command]
pub fn import_media(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    paths: Vec<String>,
) -> Result<Vec<WallpaperItem>, AppError> {
    let mut core = state.core()?;
    let imported = core.import_paths(&paths.into_iter().map(PathBuf::from).collect::<Vec<_>>())?;
    for item in &imported {
        app.asset_protocol_scope()
            .allow_file(&item.path)
            .map_err(|problem| error("preview_scope_failed", &problem.to_string(), true))?;
    }
    state.commit(&app, &core)?;
    Ok(imported)
}

#[tauri::command]
pub fn remove_media(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    media_id: String,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    let was_active = core.snapshot().playback.active_id.as_deref() == Some(media_id.as_str());
    core.remove(&media_id)?;
    if was_active {
        state.player()?.stop();
    }
    state.commit(&app, &core)
}

#[tauri::command]
pub fn relocate_media(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    media_id: String,
    path: String,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.relocate(&media_id, Path::new(&path))?;
    app.asset_protocol_scope()
        .allow_file(&path)
        .map_err(|problem| error("preview_scope_failed", &problem.to_string(), true))?;
    state.commit(&app, &core)
}

#[tauri::command]
pub fn play(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    media_id: String,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.play(&media_id)?;
    let item = core
        .snapshot()
        .library
        .iter()
        .find(|item| item.id == media_id)
        .cloned()
        .ok_or_else(|| error("wallpaper_not_found", "壁纸不在媒体库中", true))?;
    let result = state.player()?.play(
        &state.mpv_binary,
        Path::new(&item.path),
        item.kind,
        &core.snapshot().settings,
    );
    if let Err(problem) = result {
        let app_error = player_error(problem);
        state.log_error("play", &app_error.message);
        core.set_playback_error(app_error.message.clone());
        state.commit(&app, &core)?;
        return Err(app_error);
    }
    state.commit(&app, &core)
}

#[tauri::command]
pub fn toggle_pause(
    app: AppHandle,
    state: State<'_, RuntimeState>,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.toggle_pause()?;
    state
        .player()?
        .set_paused(core.snapshot().playback.is_paused())
        .map_err(player_error)?;
    state.commit(&app, &core)
}

#[tauri::command]
pub fn stop(app: AppHandle, state: State<'_, RuntimeState>) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.stop();
    state.player()?.stop();
    state.commit(&app, &core)
}

#[tauri::command]
pub fn set_muted(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    muted: bool,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.set_muted(muted);
    if core.snapshot().playback.active_id.is_some() {
        state.player()?.set_muted(muted).map_err(player_error)?;
    }
    state.commit(&app, &core)
}

#[tauri::command]
pub fn set_volume(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    volume: u8,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.set_volume(volume)?;
    if core.snapshot().playback.active_id.is_some() {
        state.player()?.set_volume(volume).map_err(player_error)?;
    }
    state.commit(&app, &core)
}

#[tauri::command]
pub fn set_scale_mode(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    mode: ScaleMode,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.set_scale_mode(mode);
    if core.snapshot().playback.active_id.is_some() {
        state
            .player()?
            .set_scale_mode(&core.snapshot().settings)
            .map_err(player_error)?;
    }
    state.commit(&app, &core)
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    settings: AppSettings,
) -> Result<AppSnapshot, AppError> {
    if settings.auto_start {
        app.autolaunch()
            .enable()
            .map_err(|problem| error("autostart_failed", &problem.to_string(), true))?;
    } else {
        app.autolaunch()
            .disable()
            .map_err(|problem| error("autostart_failed", &problem.to_string(), true))?;
    }
    let mut core = state.core()?;
    core.update_settings(settings)?;
    state.commit(&app, &core)
}

#[tauri::command]
pub fn open_media_folder(state: State<'_, RuntimeState>, media_id: String) -> Result<(), AppError> {
    let path = state.core()?.media_path(&media_id)?;
    tauri_plugin_opener::reveal_item_in_dir(path)
        .map_err(|problem| error("open_folder_failed", &problem.to_string(), true))
}

#[tauri::command]
pub fn open_logs(app: AppHandle, state: State<'_, RuntimeState>) -> Result<(), AppError> {
    let logs = state.data_dir.join("logs");
    std::fs::create_dir_all(&logs)
        .map_err(|problem| error("open_logs_failed", &problem.to_string(), true))?;
    app.opener()
        .open_path(logs.to_string_lossy(), None::<String>)
        .map_err(|problem| error("open_logs_failed", &problem.to_string(), true))
}

#[tauri::command]
pub fn open_license(app: AppHandle) -> Result<(), AppError> {
    let path = resolve_bundled_file("LICENSE");
    app.opener()
        .open_path(path.to_string_lossy(), None::<String>)
        .map_err(|problem| error("open_license_failed", &problem.to_string(), true))
}

#[tauri::command]
pub fn open_project_homepage() -> Result<(), AppError> {
    Err(error("homepage_not_configured", "项目主页尚未配置", true))
}

#[tauri::command]
pub fn quit(app: AppHandle, state: State<'_, RuntimeState>) {
    if let Ok(mut player) = state.player.lock() {
        player.stop();
    }
    app.exit(0);
}

/// 由 Windows 状态监视器设置单个自动暂停来源。
pub fn set_automatic_pause(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    reason: PauseReason,
    paused: bool,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    if core.snapshot().playback.active_id.is_none() {
        return Ok(core.snapshot().clone());
    }
    let was_paused = core.snapshot().playback.is_paused();
    core.set_pause_reason(reason, paused);
    let is_paused = core.snapshot().playback.is_paused();
    if was_paused != is_paused {
        state
            .player()?
            .set_paused(is_paused)
            .map_err(player_error)?;
    }
    state.commit(&app, &core)
}

fn resolve_mpv_binary() -> PathBuf {
    let executable_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let candidates = [
        executable_dir.join("mpv.exe"),
        executable_dir.join("mpv").join("mpv.exe"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join("mpv.exe"),
    ];
    candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .cloned()
        .unwrap_or_else(|| candidates[0].clone())
}

fn resolve_bundled_file(name: &str) -> PathBuf {
    let executable_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let bundled = executable_dir.join(name);
    if bundled.is_file() {
        bundled
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(name)
    }
}

fn player_error(problem: PlayerError) -> AppError {
    match problem {
        PlayerError::MissingBinary => error(
            "mpv_missing",
            "未找到 mpv.exe，请先运行 scripts/prepare-mpv.ps1 或使用完整便携包",
            true,
        ),
        PlayerError::MissingDesktopHost => {
            error("desktop_host_missing", &problem.to_string(), true)
        }
        PlayerError::WindowTimeout
        | PlayerError::EarlyExit(_)
        | PlayerError::Embedding(_)
        | PlayerError::Io(_) => error("playback_failed", &problem.to_string(), true),
    }
}

fn error(code: &str, message: &str, recoverable: bool) -> AppError {
    AppError {
        code: code.to_owned(),
        message: message.to_owned(),
        recoverable,
    }
}
