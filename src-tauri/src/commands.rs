//! 将 Vue 和托盘操作桥接到同一套 WallCore、持久化与 mpv 实例。

use crate::core::WallCore;
use crate::model::{
    AppError, AppSettings, AppSnapshot, DisplayAssignment, DisplayInfo, DisplayMode, PauseReason,
    ScaleMode, WallpaperItem, WallpaperSettings,
};
use crate::player::{MpvPlayerManager, PlayerError, ScreenRegion};
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

const PROJECT_HOMEPAGE: &str = "https://github.com/NiceBlueChai/wall";

pub struct RuntimeState {
    core: Mutex<WallCore>,
    commit_gate: Mutex<()>,
    player: Mutex<MpvPlayerManager>,
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
        if discard_disabled_session(&mut snapshot) {
            storage.save(&snapshot)?;
        }
        for item in &mut snapshot.library {
            item.missing = !Path::new(&item.path).is_file();
            if !item.missing {
                let _ = app.asset_protocol_scope().allow_file(&item.path);
            }
        }
        let mut core = WallCore::new(snapshot);
        core.set_displays(collect_displays(app.handle())?);
        Ok(Self {
            core: Mutex::new(core),
            commit_gate: Mutex::new(()),
            player: Mutex::new(MpvPlayerManager::default()),
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

    fn player(&self) -> Result<MutexGuard<'_, MpvPlayerManager>, AppError> {
        self.player
            .lock()
            .map_err(|_| error("player_poisoned", "播放器状态暂时不可用", true))
    }

    fn commit(
        &self,
        app: &AppHandle,
        core: MutexGuard<'_, WallCore>,
    ) -> Result<AppSnapshot, AppError> {
        let (snapshot, _commit_guard) = prepare_commit(&self.core, core, &self.commit_gate)?;
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

fn prepare_commit<'a>(
    core_mutex: &Mutex<WallCore>,
    core: MutexGuard<'_, WallCore>,
    commit_gate: &'a Mutex<()>,
) -> Result<(AppSnapshot, MutexGuard<'a, ()>), AppError> {
    drop(core);
    let commit_guard = commit_gate
        .lock()
        .map_err(|_| error("commit_poisoned", "应用状态暂时无法提交", true))?;
    let snapshot = core_mutex
        .lock()
        .map_err(|_| error("state_poisoned", "应用状态暂时不可用", false))?
        .snapshot()
        .clone();
    Ok((snapshot, commit_guard))
}

fn discard_disabled_session(snapshot: &mut AppSnapshot) -> bool {
    if snapshot.settings.restore_last_wallpaper
        || snapshot.playback == crate::model::PlaybackState::default()
    {
        return false;
    }
    snapshot.playback = crate::model::PlaybackState::default();
    true
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
    state.commit(&app, core)?;
    Ok(imported)
}

fn collect_displays(app: &AppHandle) -> tauri::Result<Vec<DisplayInfo>> {
    let primary = app.primary_monitor()?;
    Ok(app
        .available_monitors()?
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| {
            let position = monitor.position();
            let size = monitor.size();
            let name = monitor
                .name()
                .cloned()
                .unwrap_or_else(|| format!("显示器 {}", index + 1));
            let is_primary = primary
                .as_ref()
                .is_some_and(|value| value.position() == position && value.size() == size);
            DisplayInfo {
                id: monitor
                    .name()
                    .cloned()
                    .unwrap_or_else(|| format!("display-{}", index + 1)),
                name,
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
                primary: is_primary,
                connected: true,
            }
        })
        .collect())
}

/// 刷新显示器热插拔状态；仅在枚举结果变化时持久化并广播。
pub fn refresh_displays(app: AppHandle, state: State<'_, RuntimeState>) -> Result<(), AppError> {
    let displays = collect_displays(&app)
        .map_err(|problem| error("display_enumeration_failed", &problem.to_string(), true))?;
    let mut core = state.core()?;
    let before = core.snapshot().clone();
    core.set_displays(displays);
    let exited = state
        .player()?
        .take_exited_targets()
        .map_err(player_error)?;
    for target_id in exited {
        let _ = core.set_target_error(&target_id, "mpv 子进程意外退出，正在尝试恢复".to_owned());
    }
    let assignments = core.snapshot().playback.display_assignments.clone();
    for assignment in assignments {
        let connected = assignment.display_ids.iter().all(|id| {
            core.snapshot()
                .displays
                .iter()
                .any(|display| &display.id == id && display.connected)
        });
        let running = state.player()?.has_target(&assignment.target_id);
        if !connected && running {
            state.player()?.stop_target(&assignment.target_id);
        } else if connected && !running {
            match start_assignment_player(&state, &core, &assignment) {
                Ok(()) => {
                    core.restore_target_status(&assignment.target_id)?;
                }
                Err(problem) => {
                    state.log_error("display_restore", &problem.message);
                    core.set_target_error(&assignment.target_id, problem.message)?;
                }
            }
        }
    }
    if core.snapshot() != &before {
        state.commit(&app, core)?;
    }
    Ok(())
}

/// 创建用户分类并广播更新后的完整快照。
#[tauri::command]
pub fn create_category(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    name: String,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.create_category(&name)?;
    state.commit(&app, core)
}

/// 重命名用户分类并保持现有壁纸归属。
#[tauri::command]
pub fn rename_category(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    category_id: String,
    name: String,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.rename_category(&category_id, &name)?;
    state.commit(&app, core)
}

/// 删除分类并仅解除归属，不删除媒体项目或源文件。
#[tauri::command]
pub fn delete_category(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    category_id: String,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.delete_category(&category_id)?;
    state.commit(&app, core)
}

/// 批量添加或移除壁纸分类，并在提交前验证所有标识。
#[tauri::command]
pub fn set_category_membership(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    media_ids: Vec<String>,
    category_id: String,
    assigned: bool,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.set_category_membership(&media_ids, &category_id, assigned)?;
    state.commit(&app, core)
}

/// 选择独立、复制或铺展显示器组，并广播最新快照。
#[tauri::command]
pub fn set_display_layout(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    mode: DisplayMode,
    display_ids: Vec<String>,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.set_display_layout(mode, display_ids)?;
    state.commit(&app, core)
}

#[tauri::command]
pub fn remove_media(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    media_id: String,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.remove(&media_id)?;
    state.player()?.stop_media(&media_id);
    state.commit(&app, core)
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
    state.commit(&app, core)
}

#[tauri::command]
pub fn play(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    media_id: String,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    let previous_snapshot = core.snapshot().clone();
    let had_active_target = !previous_snapshot.playback.display_assignments.is_empty();
    core.play(&media_id)?;
    let item = core
        .snapshot()
        .library
        .iter()
        .find(|item| item.id == media_id)
        .cloned()
        .ok_or_else(|| error("wallpaper_not_found", "壁纸不在媒体库中", true))?;
    let settings = core.effective_settings(&media_id)?;
    let assignment = core
        .snapshot()
        .playback
        .display_assignments
        .iter()
        .rev()
        .find(|assignment| assignment.wallpaper_id == media_id)
        .cloned();
    let result = if let Some(assignment) = assignment {
        start_assignment_player(&state, &core, &assignment)
    } else {
        state
            .player()?
            .play(
                &media_id,
                &state.mpv_binary,
                Path::new(&item.path),
                item.kind,
                &settings,
            )
            .map_err(player_error)
    };
    if let Err(app_error) = result {
        state.log_error("play", &app_error.message);
        *core = WallCore::new(previous_snapshot);
        if !had_active_target {
            core.set_playback_error(app_error.message.clone());
            state.commit(&app, core)?;
        }
        return Err(app_error);
    }
    state.commit(&app, core)
}

fn start_assignment_player(
    state: &RuntimeState,
    core: &WallCore,
    assignment: &DisplayAssignment,
) -> Result<(), AppError> {
    let item = core
        .snapshot()
        .library
        .iter()
        .find(|item| item.id == assignment.wallpaper_id)
        .ok_or_else(|| error("wallpaper_not_found", "壁纸不在媒体库中", true))?;
    let settings = core.effective_settings(&assignment.wallpaper_id)?;
    let regions = assignment
        .display_ids
        .iter()
        .filter_map(|id| {
            core.snapshot()
                .displays
                .iter()
                .find(|display| &display.id == id && display.connected)
        })
        .map(|display| ScreenRegion {
            x: display.x,
            y: display.y,
            width: i32::try_from(display.width).unwrap_or(i32::MAX),
            height: i32::try_from(display.height).unwrap_or(i32::MAX),
        })
        .collect::<Vec<_>>();
    if regions.len() != assignment.display_ids.len() {
        return Err(error("display_offline", "显示目标包含离线屏幕", true));
    }
    let mut player = state.player()?;
    player
        .play_target_configured(
            &assignment.target_id,
            &assignment.wallpaper_id,
            &state.mpv_binary,
            Path::new(&item.path),
            item.kind,
            &settings,
            assignment.mode,
            &assignment.display_ids,
            &regions,
            !assignment.pause_reasons.is_empty(),
            assignment.muted,
            assignment.volume,
        )
        .map_err(player_error)
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
    state.commit(&app, core)
}

/// 切换单个显示目标的手动暂停状态。
#[tauri::command]
pub fn toggle_target_pause(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    target_id: String,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.toggle_target_pause(&target_id)?;
    let paused = core
        .snapshot()
        .playback
        .display_assignments
        .iter()
        .find(|assignment| assignment.target_id == target_id)
        .is_some_and(|assignment| assignment.status == crate::model::PlaybackStatus::Paused);
    let player = state.player()?;
    if player.has_target(&target_id)
        && let Err(problem) = player.set_target_paused(&target_id, paused)
    {
        core.toggle_target_pause(&target_id)?;
        return Err(player_error(problem));
    }
    state.commit(&app, core)
}

#[tauri::command]
pub fn stop(app: AppHandle, state: State<'_, RuntimeState>) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.stop();
    state.player()?.stop();
    state.commit(&app, core)
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
    state.commit(&app, core)
}

/// 设置单个显示目标的静音状态。
#[tauri::command]
pub fn set_target_muted(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    target_id: String,
    muted: bool,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    let previous = core
        .snapshot()
        .playback
        .display_assignments
        .iter()
        .find(|assignment| assignment.target_id == target_id)
        .map(|assignment| assignment.muted)
        .ok_or_else(|| error("display_target_not_found", "显示目标不存在", true))?;
    core.set_target_muted(&target_id, muted)?;
    let player = state.player()?;
    if player.has_target(&target_id)
        && let Err(problem) = player.set_target_muted(&target_id, muted)
    {
        core.set_target_muted(&target_id, previous)?;
        return Err(player_error(problem));
    }
    state.commit(&app, core)
}

/// 停止单个显示目标。
#[tauri::command]
pub fn stop_target(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    target_id: String,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    core.stop_target(&target_id)?;
    state.player()?.stop_target(&target_id);
    state.commit(&app, core)
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
    state.commit(&app, core)
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
    state.commit(&app, core)
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    settings: AppSettings,
) -> Result<AppSnapshot, AppError> {
    if settings.volume > 100 || ![0, 24, 30, 60].contains(&settings.frame_rate) {
        return Err(error("invalid_settings", "音量或帧率设置无效", true));
    }
    let mut core = state.core()?;
    let previous_snapshot = core.snapshot().clone();
    let previous_auto_start = previous_snapshot.settings.auto_start;
    if settings.auto_start {
        app.autolaunch()
            .enable()
            .map_err(|problem| error("autostart_failed", &problem.to_string(), true))?;
    } else {
        app.autolaunch()
            .disable()
            .map_err(|problem| error("autostart_failed", &problem.to_string(), true))?;
    }
    let active_id = core.snapshot().playback.active_id.clone();
    let mut running_media_ids = core
        .snapshot()
        .playback
        .display_assignments
        .iter()
        .map(|assignment| assignment.wallpaper_id.clone())
        .collect::<Vec<_>>();
    if let Some(id) = &active_id
        && !running_media_ids.contains(id)
    {
        running_media_ids.push(id.clone());
    }
    running_media_ids.sort();
    running_media_ids.dedup();
    let effective_settings = running_media_ids
        .iter()
        .filter_map(|id| {
            core.snapshot()
                .library
                .iter()
                .find(|item| item.id == *id)
                .map(|item| {
                    let mut effective = settings.clone();
                    item.settings.apply_to(&mut effective);
                    (id.clone(), effective)
                })
        })
        .collect::<Vec<_>>();
    let previous_effective_settings = running_media_ids
        .iter()
        .filter_map(|id| {
            core.effective_settings(id)
                .ok()
                .map(|value| (id.clone(), value))
        })
        .collect::<Vec<_>>();
    if !effective_settings.is_empty() {
        let mut player = state.player()?;
        for (media_id, effective) in &effective_settings {
            let assignments = core
                .snapshot()
                .playback
                .display_assignments
                .iter()
                .filter(|assignment| assignment.wallpaper_id == *media_id)
                .cloned()
                .collect::<Vec<_>>();
            if let Err(problem) =
                apply_media_runtime_settings(&mut player, media_id, effective, &assignments)
            {
                let app_error = player_error(problem);
                for (rollback_id, rollback_settings) in &previous_effective_settings {
                    let rollback_assignments = previous_snapshot
                        .playback
                        .display_assignments
                        .iter()
                        .filter(|assignment| assignment.wallpaper_id == *rollback_id)
                        .cloned()
                        .collect::<Vec<_>>();
                    let _ = apply_media_runtime_settings(
                        &mut player,
                        rollback_id,
                        rollback_settings,
                        &rollback_assignments,
                    );
                }
                restore_autostart(&app, previous_auto_start);
                *core = WallCore::new(previous_snapshot);
                return Err(app_error);
            }
            core.set_media_playback_settings(media_id, effective.default_muted, effective.volume)?;
        }
    }
    if core.snapshot().playback.display_assignments.is_empty()
        && let Some((_, effective)) = effective_settings
            .iter()
            .find(|(id, _)| Some(id) == active_id.as_ref())
    {
        core.set_muted(effective.default_muted);
        core.set_volume(effective.volume)?;
    }
    core.update_settings(settings)?;
    state.commit(&app, core)
}

/// 更新单张壁纸覆盖项，并在该壁纸运行时实时应用有效设置。
#[tauri::command]
pub fn set_wallpaper_settings(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    media_id: String,
    settings: WallpaperSettings,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    let previous_snapshot = core.snapshot().clone();
    let previous = core
        .snapshot()
        .library
        .iter()
        .find(|item| item.id == media_id)
        .map(|item| item.settings.clone())
        .ok_or_else(|| error("wallpaper_not_found", "壁纸不在媒体库中", true))?;
    core.update_wallpaper_settings(&media_id, settings)?;
    let effective = core.effective_settings(&media_id)?;
    let is_running = core
        .snapshot()
        .playback
        .display_assignments
        .iter()
        .any(|assignment| assignment.wallpaper_id == media_id)
        || core.snapshot().playback.active_id.as_deref() == Some(media_id.as_str());
    if is_running {
        let mut player = state.player()?;
        let assignments = core
            .snapshot()
            .playback
            .display_assignments
            .iter()
            .filter(|assignment| assignment.wallpaper_id == media_id)
            .cloned()
            .collect::<Vec<_>>();
        let result = apply_media_runtime_settings(&mut player, &media_id, &effective, &assignments);
        if let Err(problem) = result {
            let mut previous_effective = previous_snapshot.settings.clone();
            previous.apply_to(&mut previous_effective);
            let previous_assignments = previous_snapshot
                .playback
                .display_assignments
                .iter()
                .filter(|assignment| assignment.wallpaper_id == media_id)
                .cloned()
                .collect::<Vec<_>>();
            let _ = apply_media_runtime_settings(
                &mut player,
                &media_id,
                &previous_effective,
                &previous_assignments,
            );
            let app_error = player_error(problem);
            *core = WallCore::new(previous_snapshot);
            return Err(app_error);
        }
        core.set_media_playback_settings(&media_id, effective.default_muted, effective.volume)?;
    }
    state.commit(&app, core)
}

fn restore_autostart(app: &AppHandle, enabled: bool) {
    if enabled {
        let _ = app.autolaunch().enable();
    } else {
        let _ = app.autolaunch().disable();
    }
}

fn apply_media_runtime_settings(
    player: &mut MpvPlayerManager,
    media_id: &str,
    settings: &AppSettings,
    assignments: &[crate::model::DisplayAssignment],
) -> Result<(), PlayerError> {
    player.apply_media_settings(media_id, settings)?;
    for assignment in assignments {
        if !player.has_target(&assignment.target_id) {
            continue;
        }
        player.set_target_paused(
            &assignment.target_id,
            assignment.status == crate::model::PlaybackStatus::Paused,
        )?;
        player.set_target_muted(&assignment.target_id, settings.default_muted)?;
        player.set_target_volume(&assignment.target_id, settings.volume)?;
    }
    Ok(())
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

/// 将官方项目主页交给系统默认浏览器打开。
#[tauri::command]
pub fn open_project_homepage(app: AppHandle) -> Result<(), AppError> {
    app.opener()
        .open_url(PROJECT_HOMEPAGE, None::<String>)
        .map_err(|problem| error("open_homepage_failed", &problem.to_string(), true))
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
    let previous_snapshot = core.snapshot().clone();
    let before = previous_snapshot.playback.display_assignments.clone();
    let was_paused = core.snapshot().playback.is_paused();
    core.set_pause_reason(reason, paused);
    let after = core.snapshot().playback.display_assignments.clone();
    if !before.is_empty() {
        let player = state.player()?;
        let transitions =
            online_pause_transitions(pause_transitions(&before, &after), |target_id| {
                player.has_target(target_id)
            });
        let mut applied: Vec<(String, bool)> = Vec::new();
        for (target_id, target_paused) in transitions {
            if let Err(problem) = player.set_target_paused(&target_id, target_paused) {
                for (applied_id, applied_paused) in applied.iter().rev() {
                    let _ = player.set_target_paused(applied_id, !applied_paused);
                }
                *core = WallCore::new(previous_snapshot);
                return Err(player_error(problem));
            }
            applied.push((target_id, target_paused));
        }
        return state.commit(&app, core);
    }
    let is_paused = core.snapshot().playback.is_paused();
    if was_paused != is_paused
        && let Err(problem) = state.player()?.set_paused(is_paused)
    {
        *core = WallCore::new(previous_snapshot);
        return Err(player_error(problem));
    }
    state.commit(&app, core)
}

fn pause_transitions(
    before: &[crate::model::DisplayAssignment],
    after: &[crate::model::DisplayAssignment],
) -> Vec<(String, bool)> {
    after
        .iter()
        .filter_map(|assignment| {
            let was_paused = before
                .iter()
                .find(|previous| previous.target_id == assignment.target_id)
                .is_some_and(|previous| previous.status == crate::model::PlaybackStatus::Paused);
            let is_paused = assignment.status == crate::model::PlaybackStatus::Paused;
            (was_paused != is_paused).then(|| (assignment.target_id.clone(), is_paused))
        })
        .collect()
}

fn online_pause_transitions<F>(
    transitions: Vec<(String, bool)>,
    mut is_online: F,
) -> Vec<(String, bool)>
where
    F: FnMut(&str) -> bool,
{
    transitions
        .into_iter()
        .filter(|(target_id, _)| is_online(target_id))
        .collect()
}

/// 由 Windows 状态监视器更新单个显示目标的自动暂停来源。
pub fn set_target_automatic_pause(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    target_id: &str,
    reason: PauseReason,
    paused: bool,
) -> Result<AppSnapshot, AppError> {
    let mut core = state.core()?;
    let previous_snapshot = core.snapshot().clone();
    let was_paused = core
        .snapshot()
        .playback
        .display_assignments
        .iter()
        .find(|assignment| assignment.target_id == target_id)
        .is_some_and(|assignment| assignment.status == crate::model::PlaybackStatus::Paused);
    core.set_target_pause_reason(target_id, reason, paused)?;
    let is_paused = core
        .snapshot()
        .playback
        .display_assignments
        .iter()
        .find(|assignment| assignment.target_id == target_id)
        .is_some_and(|assignment| assignment.status == crate::model::PlaybackStatus::Paused);
    let player = state.player()?;
    if was_paused != is_paused
        && player.has_target(target_id)
        && let Err(problem) = player.set_target_paused(target_id, is_paused)
    {
        *core = WallCore::new(previous_snapshot);
        return Err(player_error(problem));
    }
    state.commit(&app, core)
}

/// 由后台监视器定期校准复制组播放位置。
pub fn sync_clone_players(state: State<'_, RuntimeState>) -> Result<(), AppError> {
    state.player()?.sync_clone_groups().map_err(player_error)
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

#[cfg(test)]
mod tests {
    use super::{
        PROJECT_HOMEPAGE, discard_disabled_session, online_pause_transitions, pause_transitions,
        prepare_commit,
    };
    use crate::core::WallCore;
    use crate::model::{AppSnapshot, DisplayAssignment, DisplayMode, PauseReason, PlaybackStatus};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn commit_waits_without_core_lock_and_uses_the_latest_snapshot() {
        let core = Arc::new(Mutex::new(WallCore::new(AppSnapshot::default())));
        let commit_gate = Arc::new(Mutex::new(()));
        let held_gate = commit_gate.lock().expect("commit gate");
        let worker_core = Arc::clone(&core);
        let worker_gate = Arc::clone(&commit_gate);
        let worker = thread::spawn(move || {
            let core_guard = worker_core.lock().expect("core lock");
            let (snapshot, _commit_guard) =
                prepare_commit(&worker_core, core_guard, &worker_gate).expect("prepare commit");
            snapshot
        });
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut latest = None;
        while Instant::now() < deadline {
            if let Ok(mut guard) = core.try_lock() {
                guard.set_volume(42).expect("set volume");
                latest = Some(guard.snapshot().clone());
                break;
            }
            thread::yield_now();
        }
        let latest = latest.expect("worker must release core before waiting for commit gate");
        drop(held_gate);

        assert_eq!(worker.join().expect("worker"), latest);
    }

    #[test]
    fn project_homepage_points_to_the_public_repository() {
        assert_eq!(PROJECT_HOMEPAGE, "https://github.com/NiceBlueChai/wall");
    }

    #[test]
    fn automatic_pause_updates_each_target_without_resuming_manual_pause() {
        let mut before = vec![assignment("left", vec![PauseReason::Manual])];
        before.push(assignment("right", Vec::new()));
        let mut after = before.clone();
        after[0].pause_reasons.push(PauseReason::Battery);
        after[1].pause_reasons.push(PauseReason::Battery);
        for assignment in &mut after {
            assignment.status = PlaybackStatus::Paused;
        }
        assert_eq!(
            pause_transitions(&before, &after),
            vec![("right".to_owned(), true)]
        );

        let before_resume = after.clone();
        after[0]
            .pause_reasons
            .retain(|reason| *reason != PauseReason::Battery);
        after[1].pause_reasons.clear();
        after[1].status = PlaybackStatus::Playing;
        assert_eq!(
            pause_transitions(&before_resume, &after),
            vec![("right".to_owned(), false)]
        );
    }

    #[test]
    fn disabled_session_restore_clears_persisted_running_state() {
        let mut snapshot = AppSnapshot::default();
        snapshot.settings.restore_last_wallpaper = false;
        snapshot.playback.active_id = Some("wallpaper".to_owned());
        snapshot.playback.status = PlaybackStatus::Playing;
        snapshot.playback.display_assignments = vec![assignment("primary", Vec::new())];

        assert!(discard_disabled_session(&mut snapshot));
        assert_eq!(snapshot.playback.status, PlaybackStatus::Idle);
        assert!(snapshot.playback.active_id.is_none());
        assert!(snapshot.playback.display_assignments.is_empty());
        assert!(!discard_disabled_session(&mut snapshot));
    }

    #[test]
    fn offline_targets_keep_state_without_receiving_player_commands() {
        let transitions = vec![("online".to_owned(), true), ("offline".to_owned(), true)];

        assert_eq!(
            online_pause_transitions(transitions, |target_id| target_id == "online"),
            vec![("online".to_owned(), true)]
        );
    }

    fn assignment(target_id: &str, pause_reasons: Vec<PauseReason>) -> DisplayAssignment {
        DisplayAssignment {
            target_id: target_id.to_owned(),
            mode: DisplayMode::Independent,
            display_ids: vec![target_id.to_owned()],
            wallpaper_id: "wallpaper".to_owned(),
            status: if pause_reasons.is_empty() {
                PlaybackStatus::Playing
            } else {
                PlaybackStatus::Paused
            },
            muted: true,
            volume: 0,
            pause_reasons,
        }
    }
}
