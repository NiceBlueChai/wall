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
use std::sync::{Mutex, MutexGuard, mpsc};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_opener::OpenerExt as _;

const PROJECT_HOMEPAGE: &str = "https://github.com/NiceBlueChai/wall";

struct SnapshotPublisher {
    sender: mpsc::Sender<AppSnapshot>,
}

impl SnapshotPublisher {
    fn new(app: AppHandle, data_dir: PathBuf) -> std::io::Result<Self> {
        Self::spawn(move |snapshot| {
            if let Err(problem) = app.emit("app-state://changed", &snapshot) {
                write_log(&data_dir, "publish", &problem.to_string());
            }
            if let Some(tray) = app.try_state::<TrayMenuState>() {
                tray.update(&snapshot);
            }
        })
    }

    fn spawn<F>(publish: F) -> std::io::Result<Self>
    where
        F: Fn(AppSnapshot) + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("wall-state-publisher".to_owned())
            .spawn(move || {
                for snapshot in receiver {
                    publish(snapshot);
                }
            })?;
        Ok(Self { sender })
    }

    fn publish(&self, snapshot: AppSnapshot) -> Result<(), AppError> {
        self.sender
            .send(snapshot)
            .map_err(|problem| error("publish_failed", &problem.to_string(), true))
    }
}

pub struct RuntimeState {
    core: Mutex<WallCore>,
    commit_gate: Mutex<()>,
    player: Mutex<MpvPlayerManager>,
    publisher: SnapshotPublisher,
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
        let publisher = SnapshotPublisher::new(app.handle().clone(), data_dir.clone())?;
        Ok(Self {
            core: Mutex::new(core),
            commit_gate: Mutex::new(()),
            player: Mutex::new(MpvPlayerManager::default()),
            publisher,
            storage,
            mpv_binary: resolve_mpv_binary(),
            data_dir,
        })
    }

    /// 将无法向界面返回的后台错误追加到本地日志。
    pub fn log_error(&self, context: &str, message: &str) {
        write_log(&self.data_dir, context, message);
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

    /// 在候选状态中执行纯业务变更，只有持久化成功后才替换共享状态。
    fn commit_core_change<T, F>(&self, mutate: F) -> Result<(AppSnapshot, T), AppError>
    where
        F: FnOnce(&mut WallCore) -> Result<T, AppError>,
    {
        let _commit_guard = self
            .commit_gate
            .lock()
            .map_err(|_| error("commit_poisoned", "应用状态暂时无法提交", true))?;
        let mut shared_core = self.core()?;
        let mut candidate = WallCore::new(shared_core.snapshot().clone());
        let output = mutate(&mut candidate)?;
        self.storage
            .save(candidate.snapshot())
            .map_err(|problem| error("storage_failed", &problem.to_string(), true))?;
        *shared_core = candidate;
        let snapshot = shared_core.snapshot().clone();
        if let Err(problem) = self.publisher.publish(snapshot.clone()) {
            self.log_error("publish", &problem.message);
        }
        Ok((snapshot, output))
    }

    /// 在候选状态中提交暂停原因，并在播放器或存储失败时恢复全部已应用目标。
    fn commit_pause_change<F>(&self, mutate: F) -> Result<AppSnapshot, AppError>
    where
        F: FnOnce(&mut WallCore) -> Result<(), AppError>,
    {
        let _commit_guard = self
            .commit_gate
            .lock()
            .map_err(|_| error("commit_poisoned", "应用状态暂时无法提交", true))?;
        let mut shared_core = self.core()?;
        let before = shared_core.snapshot().clone();
        let mut candidate = WallCore::new(before.clone());
        mutate(&mut candidate)?;
        if candidate.snapshot() == &before {
            return Ok(before);
        }

        let mut player = self.player()?;
        let target_changes = online_pause_transitions(
            pause_transitions(
                &before.playback.display_assignments,
                &candidate.snapshot().playback.display_assignments,
            ),
            |target_id| player.has_target(target_id),
        )
        .into_iter()
        .filter_map(|(target_id, requested)| {
            before
                .playback
                .display_assignments
                .iter()
                .find(|assignment| assignment.target_id == target_id)
                .map(|assignment| {
                    (
                        target_id,
                        assignment.status == crate::model::PlaybackStatus::Paused,
                        requested,
                    )
                })
        })
        .collect::<Vec<_>>();
        let legacy_change = (before.playback.display_assignments.is_empty()
            && candidate.snapshot().playback.display_assignments.is_empty())
        .then(|| {
            (
                before.playback.is_paused(),
                candidate.snapshot().playback.is_paused(),
            )
        })
        .filter(|(previous, requested)| previous != requested);
        let mut applied_targets = Vec::new();
        for (target_id, previous, requested) in &target_changes {
            if let Err(problem) = player.set_target_paused(target_id, *requested) {
                let mut rollback_targets = applied_targets.clone();
                rollback_targets.push((target_id.clone(), *previous));
                if !rollback_media_pause(&mut player, &rollback_targets, None) {
                    player.stop();
                    self.log_error("pause_rollback", "播放器暂停状态回滚失败，已停止全部目标");
                }
                return Err(player_error(problem));
            }
            applied_targets.push((target_id.clone(), *previous));
        }
        let mut applied_legacy = None;
        if let Some((previous, requested)) = legacy_change {
            if let Err(problem) = player.set_paused(requested) {
                let restored = rollback_media_pause(&mut player, &applied_targets, Some(previous));
                if !restored {
                    player.stop();
                    self.log_error("pause_rollback", "播放器暂停状态回滚失败，已停止全部目标");
                }
                return Err(player_error(problem));
            }
            applied_legacy = Some(previous);
        }
        if let Err(problem) = self.storage.save(candidate.snapshot()) {
            if !rollback_media_pause(&mut player, &applied_targets, applied_legacy) {
                player.stop();
                self.log_error(
                    "pause_rollback",
                    "持久化失败后无法恢复播放器，已停止全部目标",
                );
            }
            return Err(error("storage_failed", &problem.to_string(), true));
        }
        *shared_core = candidate;
        let snapshot = shared_core.snapshot().clone();
        if let Err(problem) = self.publisher.publish(snapshot.clone()) {
            self.log_error("publish", &problem.message);
        }
        Ok(snapshot)
    }

    /// 提交带可回滚播放器副作用的候选状态。
    fn commit_player_change<M, P, A, R>(
        &self,
        mutate: M,
        should_apply: P,
        apply: A,
        rollback: R,
    ) -> Result<AppSnapshot, AppError>
    where
        M: FnOnce(&mut WallCore) -> Result<(), AppError>,
        P: FnOnce(&AppSnapshot) -> bool,
        A: FnOnce(&mut MpvPlayerManager, &AppSnapshot) -> Result<(), PlayerError>,
        R: FnOnce(&mut MpvPlayerManager, &AppSnapshot) -> Result<(), PlayerError>,
    {
        let _commit_guard = self
            .commit_gate
            .lock()
            .map_err(|_| error("commit_poisoned", "应用状态暂时无法提交", true))?;
        let mut shared_core = self.core()?;
        let before = shared_core.snapshot().clone();
        let mut candidate = WallCore::new(before.clone());
        mutate(&mut candidate)?;
        if candidate.snapshot() == &before {
            return Ok(before);
        }
        if !should_apply(candidate.snapshot()) {
            self.storage
                .save(candidate.snapshot())
                .map_err(|problem| error("storage_failed", &problem.to_string(), true))?;
            *shared_core = candidate;
            let snapshot = shared_core.snapshot().clone();
            if let Err(problem) = self.publisher.publish(snapshot.clone()) {
                self.log_error("publish", &problem.message);
            }
            return Ok(snapshot);
        }

        let mut player = self.player()?;
        let mut rollback = Some(rollback);
        if let Err(problem) = apply(&mut player, candidate.snapshot()) {
            if rollback
                .take()
                .is_some_and(|restore| restore(&mut player, &before).is_err())
            {
                player.stop();
                self.log_error("player_rollback", "播放器状态回滚失败，已停止全部目标");
            }
            return Err(player_error(problem));
        }
        if let Err(problem) = self.storage.save(candidate.snapshot()) {
            if rollback
                .take()
                .is_some_and(|restore| restore(&mut player, &before).is_err())
            {
                player.stop();
                self.log_error(
                    "player_rollback",
                    "持久化失败后无法恢复播放器，已停止全部目标",
                );
            }
            return Err(error("storage_failed", &problem.to_string(), true));
        }
        drop(player);
        *shared_core = candidate;
        let snapshot = shared_core.snapshot().clone();
        if let Err(problem) = self.publisher.publish(snapshot.clone()) {
            self.log_error("publish", &problem.message);
        }
        Ok(snapshot)
    }

    /// 提交已持久化后即可安全执行的停止操作。
    fn commit_stop_change<M, S>(&self, mutate: M, stop_player: S) -> Result<AppSnapshot, AppError>
    where
        M: FnOnce(&mut WallCore) -> Result<(), AppError>,
        S: FnOnce(&mut MpvPlayerManager),
    {
        let _commit_guard = self
            .commit_gate
            .lock()
            .map_err(|_| error("commit_poisoned", "应用状态暂时无法提交", true))?;
        let mut shared_core = self.core()?;
        let before = shared_core.snapshot().clone();
        let mut candidate = WallCore::new(before.clone());
        mutate(&mut candidate)?;
        if candidate.snapshot() == &before {
            return Ok(before);
        }
        let mut player = self.player()?;
        self.storage
            .save(candidate.snapshot())
            .map_err(|problem| error("storage_failed", &problem.to_string(), true))?;
        stop_player(&mut player);
        drop(player);
        *shared_core = candidate;
        let snapshot = shared_core.snapshot().clone();
        if let Err(problem) = self.publisher.publish(snapshot.clone()) {
            self.log_error("publish", &problem.message);
        }
        Ok(snapshot)
    }

    fn commit_muted(&self, muted: bool) -> Result<AppSnapshot, AppError> {
        self.commit_player_change(
            |core| {
                core.set_muted(muted);
                Ok(())
            },
            |snapshot| snapshot.playback.active_id.is_some(),
            |player, _| player.set_muted(muted),
            recover_playback_audio,
        )
    }

    fn commit_target_muted(&self, target_id: &str, muted: bool) -> Result<AppSnapshot, AppError> {
        self.commit_player_change(
            |core| core.set_target_muted(target_id, muted),
            |_| true,
            |player, _| {
                if player.has_target(target_id) {
                    player.set_target_muted(target_id, muted)?;
                }
                Ok(())
            },
            |player, snapshot| restore_target_audio(player, snapshot, target_id),
        )
    }

    fn commit_volume(&self, volume: u8) -> Result<AppSnapshot, AppError> {
        self.commit_player_change(
            |core| core.set_volume(volume),
            |snapshot| snapshot.playback.active_id.is_some(),
            |player, _| player.set_volume(volume),
            recover_playback_audio,
        )
    }

    fn commit_scale_mode(&self, mode: ScaleMode) -> Result<AppSnapshot, AppError> {
        self.commit_player_change(
            |core| {
                core.set_scale_mode(mode);
                Ok(())
            },
            |snapshot| snapshot.playback.active_id.is_some(),
            |player, snapshot| player.set_scale_mode(&snapshot.settings),
            |player, snapshot| player.set_scale_mode(&snapshot.settings),
        )
    }

    /// 在候选状态中启动壁纸，并在播放器或存储失败时恢复此前播放快照。
    fn commit_play(&self, media_id: &str) -> Result<AppSnapshot, AppError> {
        let _commit_guard = self
            .commit_gate
            .lock()
            .map_err(|_| error("commit_poisoned", "应用状态暂时无法提交", true))?;
        let mut shared_core = self.core()?;
        let before = shared_core.snapshot().clone();
        let mut candidate = WallCore::new(before.clone());
        candidate.play(media_id)?;
        let item = candidate
            .snapshot()
            .library
            .iter()
            .find(|item| item.id == media_id)
            .cloned()
            .ok_or_else(|| error("wallpaper_not_found", "壁纸不在媒体库中", true))?;
        let settings = candidate.effective_settings(media_id)?;
        let assignment = candidate
            .snapshot()
            .playback
            .display_assignments
            .iter()
            .rev()
            .find(|assignment| assignment.wallpaper_id == media_id)
            .cloned();
        let mut player = self.player()?;
        let restore_plan =
            build_player_restore_plan(&before, |target_id| player.has_target(target_id));
        let play_result = if let Some(assignment) = assignment {
            start_assignment_player(&mut player, &self.mpv_binary, &candidate, &assignment)
        } else {
            player
                .play(
                    media_id,
                    &self.mpv_binary,
                    Path::new(&item.path),
                    item.kind,
                    &settings,
                )
                .map_err(player_error)
        };
        if let Err(problem) = play_result {
            self.log_error("play", &problem.message);
            if !restore_player_snapshot(&mut player, &self.mpv_binary, &before, &restore_plan) {
                player.stop();
                self.log_error("play_rollback", "播放器回滚失败，已停止全部目标");
            }
            return Err(problem);
        }
        if let Err(problem) = self.storage.save(candidate.snapshot()) {
            if !restore_player_snapshot(&mut player, &self.mpv_binary, &before, &restore_plan) {
                player.stop();
                self.log_error(
                    "play_rollback",
                    "持久化失败后无法恢复播放器，已停止全部目标",
                );
            }
            return Err(error("storage_failed", &problem.to_string(), true));
        }
        drop(player);
        *shared_core = candidate;
        let snapshot = shared_core.snapshot().clone();
        if let Err(problem) = self.publisher.publish(snapshot.clone()) {
            self.log_error("publish", &problem.message);
        }
        Ok(snapshot)
    }

    /// 在候选状态中刷新显示器与播放器目标，并在存储失败时恢复此前播放快照。
    fn commit_display_refresh(&self, displays: Vec<DisplayInfo>) -> Result<AppSnapshot, AppError> {
        let _commit_guard = self
            .commit_gate
            .lock()
            .map_err(|_| error("commit_poisoned", "应用状态暂时无法提交", true))?;
        let mut shared_core = self.core()?;
        let before = shared_core.snapshot().clone();
        let mut candidate = WallCore::new(before.clone());
        candidate.set_displays(displays);
        let mut player = self.player()?;
        let mut restore_plan =
            build_player_restore_plan(&before, |target_id| player.has_target(target_id));
        let exited = match player.take_exited_targets() {
            Ok(targets) => targets,
            Err(problem) => {
                if !restore_player_snapshot(&mut player, &self.mpv_binary, &before, &restore_plan) {
                    player.stop();
                    self.log_error("display_rollback", "显示目标检查失败且播放器无法恢复");
                }
                return Err(player_error(problem));
            }
        };
        for target_id in exited {
            restore_plan
                .target_ids
                .retain(|running_id| running_id != &target_id);
            let _ = candidate
                .set_target_error(&target_id, "mpv 子进程意外退出，正在尝试恢复".to_owned());
        }
        let assignments = candidate.snapshot().playback.display_assignments.clone();
        for assignment in assignments {
            let connected = assignment.display_ids.iter().all(|id| {
                candidate
                    .snapshot()
                    .displays
                    .iter()
                    .any(|display| &display.id == id && display.connected)
            });
            let running = player.has_target(&assignment.target_id);
            if !connected && running {
                player.stop_target(&assignment.target_id);
            } else if connected && !running {
                match start_assignment_player(
                    &mut player,
                    &self.mpv_binary,
                    &candidate,
                    &assignment,
                ) {
                    Ok(()) => {
                        if let Err(problem) = candidate.restore_target_status(&assignment.target_id)
                        {
                            if !restore_player_snapshot(
                                &mut player,
                                &self.mpv_binary,
                                &before,
                                &restore_plan,
                            ) {
                                player.stop();
                                self.log_error(
                                    "display_rollback",
                                    "显示目标状态无效且播放器无法恢复",
                                );
                            }
                            return Err(problem);
                        }
                    }
                    Err(problem) => {
                        self.log_error("display_restore", &problem.message);
                        if let Err(state_problem) =
                            candidate.set_target_error(&assignment.target_id, problem.message)
                        {
                            if !restore_player_snapshot(
                                &mut player,
                                &self.mpv_binary,
                                &before,
                                &restore_plan,
                            ) {
                                player.stop();
                                self.log_error(
                                    "display_rollback",
                                    "显示目标错误状态无效且播放器无法恢复",
                                );
                            }
                            return Err(state_problem);
                        }
                    }
                }
            }
        }
        if candidate.snapshot() == &before {
            return Ok(before);
        }
        if let Err(problem) = self.storage.save(candidate.snapshot()) {
            if !restore_player_snapshot(&mut player, &self.mpv_binary, &before, &restore_plan) {
                player.stop();
                self.log_error("display_rollback", "显示状态持久化失败且播放器无法恢复");
            }
            return Err(error("storage_failed", &problem.to_string(), true));
        }
        drop(player);
        *shared_core = candidate;
        let snapshot = shared_core.snapshot().clone();
        if let Err(problem) = self.publisher.publish(snapshot.clone()) {
            self.log_error("publish", &problem.message);
        }
        Ok(snapshot)
    }

    /// 在共享状态外构建媒体库候选快照，持久化成功后再切换状态并停止已移除媒体。
    fn commit_library_change<F>(&self, mutate: F) -> Result<AppSnapshot, AppError>
    where
        F: FnOnce(&mut WallCore) -> Result<Vec<String>, AppError>,
    {
        let _commit_guard = self
            .commit_gate
            .lock()
            .map_err(|_| error("commit_poisoned", "应用状态暂时无法提交", true))?;
        let mut shared_core = self.core()?;
        let mut candidate = WallCore::new(shared_core.snapshot().clone());
        let removed_media_ids = mutate(&mut candidate)?;
        let mut player = if removed_media_ids.is_empty() {
            None
        } else {
            Some(self.player()?)
        };
        self.storage
            .save(candidate.snapshot())
            .map_err(|problem| error("storage_failed", &problem.to_string(), true))?;
        if let Some(player) = player.as_mut() {
            for media_id in &removed_media_ids {
                player.stop_media(media_id);
            }
        }
        *shared_core = candidate;
        let snapshot = shared_core.snapshot().clone();
        if let Err(problem) = self.publisher.publish(snapshot.clone()) {
            self.log_error("publish", &problem.message);
        }
        Ok(snapshot)
    }

    /// 原子地切换一张壁纸的全部显示目标，并在持久化失败时恢复播放器状态。
    fn commit_media_pause(&self, media_id: &str) -> Result<AppSnapshot, AppError> {
        let _commit_guard = self
            .commit_gate
            .lock()
            .map_err(|_| error("commit_poisoned", "应用状态暂时无法提交", true))?;
        let mut shared_core = self.core()?;
        let before = shared_core.snapshot().clone();
        let mut candidate = WallCore::new(before.clone());
        candidate.toggle_media_pause(media_id)?;
        let target_changes = candidate
            .snapshot()
            .playback
            .display_assignments
            .iter()
            .filter(|assignment| assignment.wallpaper_id == media_id)
            .filter_map(|assignment| {
                before
                    .playback
                    .display_assignments
                    .iter()
                    .find(|previous| previous.target_id == assignment.target_id)
                    .map(|previous| {
                        (
                            assignment.target_id.clone(),
                            previous.status == crate::model::PlaybackStatus::Paused,
                            assignment.status == crate::model::PlaybackStatus::Paused,
                        )
                    })
            })
            .collect::<Vec<_>>();
        let legacy_change = target_changes.is_empty().then(|| {
            (
                before.playback.is_paused(),
                candidate.snapshot().playback.is_paused(),
            )
        });
        let mut player = self.player()?;
        let mut applied_targets = Vec::new();
        for (target_id, previous, requested) in &target_changes {
            if !player.has_target(target_id) {
                continue;
            }
            if let Err(problem) = player.set_target_paused(target_id, *requested) {
                let mut rollback_targets = applied_targets.clone();
                rollback_targets.push((target_id.clone(), *previous));
                if !rollback_media_pause(&mut player, &rollback_targets, None) {
                    player.stop_media(media_id);
                    self.log_error("pause_rollback", "播放器暂停状态回滚失败，已停止相关目标");
                }
                return Err(player_error(problem));
            }
            applied_targets.push((target_id.clone(), *previous));
        }
        let mut applied_legacy = None;
        if let Some((previous, requested)) = legacy_change {
            if let Err(problem) = player.set_paused(requested) {
                let _ = player.set_paused(previous);
                return Err(player_error(problem));
            }
            applied_legacy = Some(previous);
        }
        if let Err(problem) = self.storage.save(candidate.snapshot()) {
            if !rollback_media_pause(&mut player, &applied_targets, applied_legacy) {
                player.stop_media(media_id);
                self.log_error(
                    "pause_rollback",
                    "持久化失败后无法恢复播放器，已停止相关目标",
                );
            }
            return Err(error("storage_failed", &problem.to_string(), true));
        }
        *shared_core = candidate;
        let snapshot = shared_core.snapshot().clone();
        if let Err(problem) = self.publisher.publish(snapshot.clone()) {
            self.log_error("publish", &problem.message);
        }
        Ok(snapshot)
    }

    /// 原子地停止一张壁纸的全部目标，并保留其他壁纸的播放状态。
    fn commit_media_stop(&self, media_id: &str) -> Result<AppSnapshot, AppError> {
        self.commit_stop_change(
            |core| core.stop_media_playback(media_id),
            |player| player.stop_media(media_id),
        )
    }

    /// 原子地保存全局设置，并在持久化或播放器更新失败时恢复外部状态。
    fn commit_settings(
        &self,
        app: &AppHandle,
        settings: AppSettings,
    ) -> Result<AppSnapshot, AppError> {
        let _commit_guard = self
            .commit_gate
            .lock()
            .map_err(|_| error("commit_poisoned", "应用状态暂时无法提交", true))?;
        let mut shared_core = self.core()?;
        let previous_snapshot = shared_core.snapshot().clone();
        let mut candidate = WallCore::new(previous_snapshot.clone());
        let mut running_media_ids = candidate
            .snapshot()
            .playback
            .display_assignments
            .iter()
            .map(|assignment| assignment.wallpaper_id.clone())
            .collect::<Vec<_>>();
        if let Some(id) = &candidate.snapshot().playback.active_id
            && !running_media_ids.contains(id)
        {
            running_media_ids.push(id.clone());
        }
        running_media_ids.sort();
        running_media_ids.dedup();
        let previous_effective_settings = running_media_ids
            .iter()
            .filter_map(|id| {
                candidate
                    .effective_settings(id)
                    .ok()
                    .map(|value| (id.clone(), value))
            })
            .collect::<Vec<_>>();
        candidate.update_settings(settings.clone())?;
        let effective_settings = running_media_ids
            .iter()
            .filter_map(|id| {
                candidate
                    .effective_settings(id)
                    .ok()
                    .map(|value| (id.clone(), value))
            })
            .collect::<Vec<_>>();
        for (media_id, effective) in &effective_settings {
            candidate.set_media_playback_settings(
                media_id,
                effective.default_muted,
                effective.volume,
            )?;
        }
        let mut player = if effective_settings.is_empty() {
            None
        } else {
            Some(self.player()?)
        };
        let previous_auto_start = previous_snapshot.settings.auto_start;
        let auto_start_updated = sync_autostart(app, previous_auto_start, settings.auto_start)?;
        if let Some(player) = player.as_mut() {
            for (media_id, effective) in &effective_settings {
                let assignments = candidate
                    .snapshot()
                    .playback
                    .display_assignments
                    .iter()
                    .filter(|assignment| assignment.wallpaper_id == *media_id)
                    .cloned()
                    .collect::<Vec<_>>();
                if let Err(problem) =
                    apply_media_runtime_settings(player, media_id, effective, &assignments)
                {
                    recover_settings_side_effects(
                        player,
                        &previous_effective_settings,
                        &previous_snapshot,
                    );
                    if auto_start_updated {
                        restore_autostart(app, previous_auto_start);
                    }
                    return Err(player_error(problem));
                }
            }
        }
        if let Err(problem) = self.storage.save(candidate.snapshot()) {
            if let Some(player) = player.as_mut() {
                recover_settings_side_effects(
                    player,
                    &previous_effective_settings,
                    &previous_snapshot,
                );
            }
            if auto_start_updated {
                restore_autostart(app, previous_auto_start);
            }
            return Err(error("storage_failed", &problem.to_string(), true));
        }
        *shared_core = candidate;
        let snapshot = shared_core.snapshot().clone();
        if let Err(problem) = self.publisher.publish(snapshot.clone()) {
            self.log_error("publish", &problem.message);
        }
        Ok(snapshot)
    }

    /// 原子地保存壁纸覆盖设置，并在失败时恢复该壁纸的播放器配置。
    fn commit_wallpaper_settings(
        &self,
        media_id: &str,
        settings: WallpaperSettings,
    ) -> Result<AppSnapshot, AppError> {
        let _commit_guard = self
            .commit_gate
            .lock()
            .map_err(|_| error("commit_poisoned", "应用状态暂时无法提交", true))?;
        let mut shared_core = self.core()?;
        let previous_snapshot = shared_core.snapshot().clone();
        let mut candidate = WallCore::new(previous_snapshot.clone());
        let previous_effective = candidate.effective_settings(media_id)?;
        candidate.update_wallpaper_settings(media_id, settings)?;
        let effective = candidate.effective_settings(media_id)?;
        let assignments = candidate
            .snapshot()
            .playback
            .display_assignments
            .iter()
            .filter(|assignment| assignment.wallpaper_id == media_id)
            .cloned()
            .collect::<Vec<_>>();
        let is_running = !assignments.is_empty()
            || candidate.snapshot().playback.active_id.as_deref() == Some(media_id);
        if is_running {
            candidate.set_media_playback_settings(
                media_id,
                effective.default_muted,
                effective.volume,
            )?;
        }
        let mut player = if is_running {
            Some(self.player()?)
        } else {
            None
        };
        if let Some(player) = player.as_mut()
            && let Err(problem) =
                apply_media_runtime_settings(player, media_id, &effective, &assignments)
        {
            recover_wallpaper_settings_side_effect(
                player,
                media_id,
                &previous_effective,
                &previous_snapshot,
            );
            return Err(player_error(problem));
        }
        if let Err(problem) = self.storage.save(candidate.snapshot()) {
            if let Some(player) = player.as_mut() {
                recover_wallpaper_settings_side_effect(
                    player,
                    media_id,
                    &previous_effective,
                    &previous_snapshot,
                );
            }
            return Err(error("storage_failed", &problem.to_string(), true));
        }
        *shared_core = candidate;
        let snapshot = shared_core.snapshot().clone();
        if let Err(problem) = self.publisher.publish(snapshot.clone()) {
            self.log_error("publish", &problem.message);
        }
        Ok(snapshot)
    }
}

fn rollback_media_pause(
    player: &mut MpvPlayerManager,
    targets: &[(String, bool)],
    legacy: Option<bool>,
) -> bool {
    let mut restored = true;
    for (target_id, paused) in targets.iter().rev() {
        restored &= player.set_target_paused(target_id, *paused).is_ok();
    }
    if let Some(paused) = legacy {
        restored &= player.set_paused(paused).is_ok();
    }
    restored
}

fn recover_playback_audio(
    player: &mut MpvPlayerManager,
    snapshot: &AppSnapshot,
) -> Result<(), PlayerError> {
    if snapshot.playback.display_assignments.is_empty() {
        player.set_muted(snapshot.playback.muted)?;
        player.set_volume(snapshot.playback.volume)?;
        return Ok(());
    }
    for assignment in &snapshot.playback.display_assignments {
        if player.has_target(&assignment.target_id) {
            player.set_target_muted(&assignment.target_id, assignment.muted)?;
            player.set_target_volume(&assignment.target_id, assignment.volume)?;
        }
    }
    Ok(())
}

fn restore_target_audio(
    player: &mut MpvPlayerManager,
    snapshot: &AppSnapshot,
    target_id: &str,
) -> Result<(), PlayerError> {
    let Some(assignment) = snapshot
        .playback
        .display_assignments
        .iter()
        .find(|assignment| assignment.target_id == target_id)
    else {
        return Ok(());
    };
    if player.has_target(target_id) {
        player.set_target_muted(target_id, assignment.muted)?;
        player.set_target_volume(target_id, assignment.volume)?;
    }
    Ok(())
}

fn write_log(data_dir: &Path, context: &str, message: &str) {
    let logs = data_dir.join("logs");
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
    let (_, imported) = state.commit_core_change(|core| {
        let imported =
            core.import_paths(&paths.into_iter().map(PathBuf::from).collect::<Vec<_>>())?;
        for item in &imported {
            app.asset_protocol_scope()
                .allow_file(&item.path)
                .map_err(|problem| error("preview_scope_failed", &problem.to_string(), true))?;
        }
        Ok(imported)
    })?;
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
    state.commit_display_refresh(displays)?;
    Ok(())
}

/// 创建用户分类并广播更新后的完整快照。
#[tauri::command]
pub fn create_category(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    name: String,
) -> Result<AppSnapshot, AppError> {
    state
        .commit_core_change(|core| core.create_category(&name))
        .map(|(snapshot, _)| snapshot)
}

/// 重命名用户分类并保持现有壁纸归属。
#[tauri::command]
pub fn rename_category(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    category_id: String,
    name: String,
) -> Result<AppSnapshot, AppError> {
    state
        .commit_core_change(|core| core.rename_category(&category_id, &name))
        .map(|(snapshot, _)| snapshot)
}

/// 删除分类并仅解除归属，不删除媒体项目或源文件。
#[tauri::command]
pub fn delete_category(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    category_id: String,
) -> Result<AppSnapshot, AppError> {
    state
        .commit_core_change(|core| core.delete_category(&category_id))
        .map(|(snapshot, _)| snapshot)
}

/// 批量添加或移除壁纸分类，并在提交前验证所有标识。
#[tauri::command]
pub fn set_category_membership(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    media_ids: Vec<String>,
    category_id: String,
    assigned: bool,
) -> Result<AppSnapshot, AppError> {
    state
        .commit_core_change(|core| core.set_category_membership(&media_ids, &category_id, assigned))
        .map(|(snapshot, _)| snapshot)
}

/// 选择独立、复制或铺展显示器组，并广播最新快照。
#[tauri::command]
pub fn set_display_layout(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    mode: DisplayMode,
    display_ids: Vec<String>,
) -> Result<AppSnapshot, AppError> {
    state
        .commit_core_change(|core| core.set_display_layout(mode, display_ids))
        .map(|(snapshot, _)| snapshot)
}

#[tauri::command]
pub fn remove_media(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    media_id: String,
) -> Result<AppSnapshot, AppError> {
    state.commit_library_change(|core| {
        core.remove(&media_id)?;
        Ok(vec![media_id])
    })
}

/// 原子地从媒体库移除多个项目，并停止使用这些项目的播放目标。
#[tauri::command]
pub fn remove_media_batch(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    media_ids: Vec<String>,
) -> Result<AppSnapshot, AppError> {
    state.commit_library_change(|core| {
        core.remove_many(&media_ids)?;
        Ok(media_ids)
    })
}

/// 重新扫描全部媒体路径并广播最新失效状态。
#[tauri::command]
pub fn scan_library(
    app: AppHandle,
    state: State<'_, RuntimeState>,
) -> Result<AppSnapshot, AppError> {
    state.commit_library_change(|core| {
        core.refresh_missing();
        for item in core.snapshot().library.iter().filter(|item| !item.missing) {
            app.asset_protocol_scope()
                .allow_file(&item.path)
                .map_err(|problem| error("preview_scope_failed", &problem.to_string(), true))?;
        }
        Ok(Vec::new())
    })
}

/// 重新扫描后移除全部失效记录，不删除或修改任何源文件。
#[tauri::command]
pub fn remove_missing_media(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
) -> Result<AppSnapshot, AppError> {
    state.commit_library_change(|core| {
        let media_ids = core.refresh_missing();
        if !media_ids.is_empty() {
            core.remove_many(&media_ids)?;
        }
        Ok(media_ids)
    })
}

#[tauri::command]
pub fn relocate_media(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    media_id: String,
    path: String,
) -> Result<AppSnapshot, AppError> {
    state
        .commit_core_change(|core| {
            core.relocate(&media_id, Path::new(&path))?;
            app.asset_protocol_scope()
                .allow_file(&path)
                .map_err(|problem| error("preview_scope_failed", &problem.to_string(), true))
        })
        .map(|(snapshot, _)| snapshot)
}

#[tauri::command]
pub fn play(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    media_id: String,
) -> Result<AppSnapshot, AppError> {
    state.commit_play(&media_id)
}

fn start_assignment_player(
    player: &mut MpvPlayerManager,
    mpv_binary: &Path,
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
    player
        .play_target_configured(
            &assignment.target_id,
            &assignment.wallpaper_id,
            mpv_binary,
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

#[derive(Debug, Default, Eq, PartialEq)]
struct PlayerRestorePlan {
    target_ids: Vec<String>,
    legacy: bool,
}

fn build_player_restore_plan<F>(snapshot: &AppSnapshot, mut is_running: F) -> PlayerRestorePlan
where
    F: FnMut(&str) -> bool,
{
    let target_ids = snapshot
        .playback
        .display_assignments
        .iter()
        .filter(|assignment| {
            assignment.status != crate::model::PlaybackStatus::Error
                && is_running(&assignment.target_id)
        })
        .map(|assignment| assignment.target_id.clone())
        .collect();
    let legacy = snapshot.playback.display_assignments.is_empty()
        && snapshot.playback.status != crate::model::PlaybackStatus::Error
        && snapshot.playback.active_id.is_some()
        && is_running("display:primary");
    PlayerRestorePlan { target_ids, legacy }
}

fn restore_player_snapshot(
    player: &mut MpvPlayerManager,
    mpv_binary: &Path,
    snapshot: &AppSnapshot,
    plan: &PlayerRestorePlan,
) -> bool {
    player.stop();
    let core = WallCore::new(snapshot.clone());
    if !snapshot.playback.display_assignments.is_empty() {
        for assignment in &snapshot.playback.display_assignments {
            if !plan.target_ids.contains(&assignment.target_id) {
                continue;
            }
            let connected = assignment.display_ids.iter().all(|id| {
                snapshot
                    .displays
                    .iter()
                    .any(|display| &display.id == id && display.connected)
            });
            if connected && start_assignment_player(player, mpv_binary, &core, assignment).is_err()
            {
                player.stop();
                return false;
            }
        }
        return true;
    }
    if !plan.legacy {
        return true;
    }
    let Some(media_id) = snapshot.playback.active_id.as_deref() else {
        return true;
    };
    let Some(item) = snapshot.library.iter().find(|item| item.id == media_id) else {
        return false;
    };
    let Ok(settings) = core.effective_settings(media_id) else {
        return false;
    };
    if player
        .play(
            media_id,
            mpv_binary,
            Path::new(&item.path),
            item.kind,
            &settings,
        )
        .is_err()
    {
        player.stop();
        return false;
    }
    true
}

#[tauri::command]
pub fn toggle_pause(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
) -> Result<AppSnapshot, AppError> {
    state.commit_pause_change(WallCore::toggle_pause)
}

/// 切换单个显示目标的手动暂停状态。
#[tauri::command]
pub fn toggle_target_pause(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    target_id: String,
) -> Result<AppSnapshot, AppError> {
    state.commit_pause_change(|core| core.toggle_target_pause(&target_id))
}

/// 原子地切换使用指定壁纸的全部目标的手动暂停状态。
#[tauri::command]
pub fn toggle_media_pause(
    state: State<'_, RuntimeState>,
    media_id: String,
) -> Result<AppSnapshot, AppError> {
    state.commit_media_pause(&media_id)
}

#[tauri::command]
pub fn stop(_app: AppHandle, state: State<'_, RuntimeState>) -> Result<AppSnapshot, AppError> {
    state.commit_stop_change(
        |core| {
            core.stop();
            Ok(())
        },
        MpvPlayerManager::stop,
    )
}

#[tauri::command]
pub fn set_muted(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    muted: bool,
) -> Result<AppSnapshot, AppError> {
    state.commit_muted(muted)
}

/// 设置单个显示目标的静音状态。
#[tauri::command]
pub fn set_target_muted(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    target_id: String,
    muted: bool,
) -> Result<AppSnapshot, AppError> {
    state.commit_target_muted(&target_id, muted)
}

/// 停止单个显示目标。
#[tauri::command]
pub fn stop_target(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    target_id: String,
) -> Result<AppSnapshot, AppError> {
    state.commit_stop_change(
        |core| core.stop_target(&target_id),
        |player| player.stop_target(&target_id),
    )
}

/// 原子地停止使用指定壁纸的全部目标。
#[tauri::command]
pub fn stop_media(
    state: State<'_, RuntimeState>,
    media_id: String,
) -> Result<AppSnapshot, AppError> {
    state.commit_media_stop(&media_id)
}

#[tauri::command]
pub fn set_volume(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    volume: u8,
) -> Result<AppSnapshot, AppError> {
    state.commit_volume(volume)
}

#[tauri::command]
pub fn set_scale_mode(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    mode: ScaleMode,
) -> Result<AppSnapshot, AppError> {
    state.commit_scale_mode(mode)
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
    state.commit_settings(&app, settings)
}

/// 更新单张壁纸覆盖项，并在该壁纸运行时实时应用有效设置。
#[tauri::command]
pub fn set_wallpaper_settings(
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    media_id: String,
    settings: WallpaperSettings,
) -> Result<AppSnapshot, AppError> {
    state.commit_wallpaper_settings(&media_id, settings)
}

fn autostart_transition(previous: bool, requested: bool) -> Option<bool> {
    (previous != requested).then_some(requested)
}

fn sync_autostart(app: &AppHandle, previous: bool, requested: bool) -> Result<bool, AppError> {
    let Some(enabled) = autostart_transition(previous, requested) else {
        return Ok(false);
    };
    let manager = app.autolaunch();
    let current = manager
        .is_enabled()
        .map_err(|problem| error("autostart_failed", &problem.to_string(), true))?;
    if current == enabled {
        return Ok(false);
    }
    if enabled {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|problem| error("autostart_failed", &problem.to_string(), true))?;
    Ok(true)
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

fn recover_settings_side_effects(
    player: &mut MpvPlayerManager,
    previous_settings: &[(String, AppSettings)],
    previous_snapshot: &AppSnapshot,
) {
    for (media_id, settings) in previous_settings {
        let assignments = previous_snapshot
            .playback
            .display_assignments
            .iter()
            .filter(|assignment| assignment.wallpaper_id == *media_id)
            .cloned()
            .collect::<Vec<_>>();
        if apply_media_runtime_settings(player, media_id, settings, &assignments).is_err() {
            player.stop_media(media_id);
        }
    }
}

fn recover_wallpaper_settings_side_effect(
    player: &mut MpvPlayerManager,
    media_id: &str,
    previous_settings: &AppSettings,
    previous_snapshot: &AppSnapshot,
) {
    let assignments = previous_snapshot
        .playback
        .display_assignments
        .iter()
        .filter(|assignment| assignment.wallpaper_id == media_id)
        .cloned()
        .collect::<Vec<_>>();
    if apply_media_runtime_settings(player, media_id, previous_settings, &assignments).is_err() {
        player.stop_media(media_id);
    }
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
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    reason: PauseReason,
    paused: bool,
) -> Result<AppSnapshot, AppError> {
    state.commit_pause_change(|core| {
        if core.snapshot().playback.active_id.is_some() {
            core.set_pause_reason(reason, paused);
        }
        Ok(())
    })
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
    _app: AppHandle,
    state: State<'_, RuntimeState>,
    target_id: &str,
    reason: PauseReason,
    paused: bool,
) -> Result<AppSnapshot, AppError> {
    state.commit_pause_change(|core| core.set_target_pause_reason(target_id, reason, paused))
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
        | PlayerError::Command(_)
        | PlayerError::Rollback(_)
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
        PROJECT_HOMEPAGE, RuntimeState, SnapshotPublisher, autostart_transition,
        build_player_restore_plan, discard_disabled_session, online_pause_transitions,
        pause_transitions,
    };
    use crate::core::WallCore;
    use crate::media::import_media;
    use crate::model::{
        AppSnapshot, DisplayAssignment, DisplayMode, PauseReason, PlaybackStatus, WallpaperSettings,
    };
    use crate::player::MpvPlayerManager;
    use crate::storage::Storage;
    use std::fs;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::path::PathBuf;
    use std::sync::{Mutex, mpsc};

    #[test]
    fn unrelated_setting_changes_do_not_touch_autostart() {
        assert_eq!(autostart_transition(false, false), None);
        assert_eq!(autostart_transition(true, true), None);
        assert_eq!(autostart_transition(false, true), Some(true));
        assert_eq!(autostart_transition(true, false), Some(false));
    }

    #[test]
    fn snapshot_publisher_preserves_commit_order() {
        let (observed_sender, observed_receiver) = mpsc::channel();
        let publisher = SnapshotPublisher::spawn(move |snapshot| {
            observed_sender
                .send(snapshot.playback.volume)
                .expect("observe snapshot");
        })
        .expect("publisher thread");
        let mut first = AppSnapshot::default();
        first.playback.volume = 10;
        let mut second = AppSnapshot::default();
        second.playback.volume = 20;

        publisher.publish(first).expect("publish first");
        publisher.publish(second).expect("publish second");

        assert_eq!(observed_receiver.recv().expect("first snapshot"), 10);
        assert_eq!(observed_receiver.recv().expect("second snapshot"), 20);
    }

    #[test]
    fn library_change_keeps_core_unchanged_when_player_or_storage_is_unavailable() {
        let root = command_test_root("library-change-rollback");
        let (snapshot, media_id, source) = snapshot_with_media(&root);
        let poisoned_state = runtime_state(root.join("poisoned-storage"), snapshot.clone());
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _player = poisoned_state.player.lock().expect("player lock");
            panic!("poison player lock");
        }));

        let player_error = poisoned_state
            .commit_library_change(|core| {
                core.remove_many(std::slice::from_ref(&media_id))?;
                Ok(vec![media_id.clone()])
            })
            .expect_err("reject poisoned player");

        assert_eq!(player_error.code, "player_poisoned");
        assert_eq!(
            poisoned_state.core().expect("core state").snapshot(),
            &snapshot
        );

        let invalid_storage = root.join("storage-file");
        fs::write(&invalid_storage, b"not a directory").expect("create invalid storage root");
        let storage_state = runtime_state(invalid_storage, snapshot.clone());
        let storage_error = storage_state
            .commit_library_change(|core| {
                core.remove_many(std::slice::from_ref(&media_id))?;
                Ok(vec![media_id.clone()])
            })
            .expect_err("reject failed persistence");

        assert_eq!(storage_error.code, "storage_failed");
        assert_eq!(
            storage_state.core().expect("core state").snapshot(),
            &snapshot
        );
        assert!(source.is_file());
        fs::remove_dir_all(root).expect("clean command test directory");
    }

    #[test]
    fn library_change_returns_the_committed_snapshot_when_async_publish_is_closed() {
        let root = command_test_root("library-change-publish");
        let (snapshot, media_id, source) = snapshot_with_media(&root);
        let mut state = runtime_state(root.join("storage"), snapshot);
        let (sender, receiver) = mpsc::channel();
        drop(receiver);
        state.publisher = SnapshotPublisher { sender };

        let committed = state
            .commit_library_change(|core| {
                core.remove_many(std::slice::from_ref(&media_id))?;
                Ok(vec![media_id.clone()])
            })
            .expect("commit despite closed async publisher");

        assert!(committed.library.is_empty());
        assert!(
            state
                .core()
                .expect("core state")
                .snapshot()
                .library
                .is_empty()
        );
        assert!(
            Storage::new(root.join("storage"))
                .load()
                .expect("load committed storage")
                .library
                .is_empty()
        );
        assert!(source.is_file());
        fs::remove_dir_all(root).expect("clean command test directory");
    }

    #[test]
    fn core_change_is_atomic_on_storage_failure_and_tolerates_publish_failure() {
        let root = command_test_root("core-change-atomic");
        let invalid_storage = root.join("storage-file");
        fs::create_dir_all(&root).expect("create command test root");
        fs::write(&invalid_storage, b"not a directory").expect("create invalid storage root");
        let snapshot = AppSnapshot::default();
        let failed_state = runtime_state(invalid_storage, snapshot.clone());

        let failure = failed_state
            .commit_core_change(|core| core.create_category("自然风景"))
            .expect_err("reject failed persistence");

        assert_eq!(failure.code, "storage_failed");
        assert_eq!(
            failed_state.core().expect("core state").snapshot(),
            &snapshot
        );

        let mut published_state = runtime_state(root.join("storage"), snapshot);
        let (sender, receiver) = mpsc::channel();
        drop(receiver);
        published_state.publisher = SnapshotPublisher { sender };
        let (committed, category) = published_state
            .commit_core_change(|core| core.create_category("城市夜景"))
            .expect("commit despite closed publisher");

        assert_eq!(category.name, "城市夜景");
        assert_eq!(committed.categories.len(), 1);
        assert_eq!(
            Storage::new(root.join("storage"))
                .load()
                .expect("load committed snapshot")
                .categories
                .len(),
            1
        );
        fs::remove_dir_all(root).expect("clean command test directory");
    }

    #[test]
    fn wallpaper_settings_keep_shared_state_when_persistence_fails() {
        let root = command_test_root("wallpaper-settings-rollback");
        let (snapshot, media_id, source) = snapshot_with_media(&root);
        let invalid_storage = root.join("storage-file");
        fs::write(&invalid_storage, b"not a directory").expect("create invalid storage root");
        let state = runtime_state(invalid_storage, snapshot.clone());

        let failure = state
            .commit_wallpaper_settings(
                &media_id,
                WallpaperSettings {
                    volume: Some(42),
                    ..Default::default()
                },
            )
            .expect_err("reject failed settings persistence");

        assert_eq!(failure.code, "storage_failed");
        assert_eq!(state.core().expect("core state").snapshot(), &snapshot);
        assert!(source.is_file());
        fs::remove_dir_all(root).expect("clean command test directory");
    }

    #[test]
    fn pause_change_keeps_shared_state_when_persistence_fails() {
        let root = command_test_root("pause-change-rollback");
        let (mut snapshot, media_id, source) = snapshot_with_media(&root);
        snapshot.playback.active_id = Some(media_id.clone());
        snapshot.playback.status = PlaybackStatus::Playing;
        snapshot.playback.display_assignments = vec![DisplayAssignment {
            target_id: "display:left".to_owned(),
            mode: DisplayMode::Independent,
            display_ids: vec!["left".to_owned()],
            wallpaper_id: media_id,
            status: PlaybackStatus::Playing,
            muted: true,
            volume: 0,
            pause_reasons: Vec::new(),
        }];
        snapshot.displays = vec![crate::model::DisplayInfo {
            id: "left".to_owned(),
            name: "Left".to_owned(),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            primary: true,
            connected: true,
        }];
        let poisoned_state = runtime_state(root.join("poisoned-storage"), snapshot.clone());
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _player = poisoned_state.player.lock().expect("player lock");
            panic!("poison player lock");
        }));
        let player_failure = poisoned_state
            .commit_pause_change(WallCore::toggle_pause)
            .expect_err("reject unavailable player");
        assert_eq!(player_failure.code, "player_poisoned");
        assert_eq!(
            poisoned_state.core().expect("core state").snapshot(),
            &snapshot
        );

        let invalid_storage = root.join("storage-file");
        fs::write(&invalid_storage, b"not a directory").expect("create invalid storage root");
        let state = runtime_state(invalid_storage, snapshot.clone());

        let failure = state
            .commit_pause_change(|core| core.toggle_target_pause("display:left"))
            .expect_err("reject failed pause persistence");

        assert_eq!(failure.code, "storage_failed");
        assert_eq!(state.core().expect("core state").snapshot(), &snapshot);
        assert!(source.is_file());
        fs::remove_dir_all(root).expect("clean command test directory");
    }

    #[test]
    fn runtime_commands_never_publish_unpersisted_shared_state() {
        let root = command_test_root("runtime-command-atomic");
        let (mut snapshot, media_id, source) = snapshot_with_media(&root);
        snapshot.playback.active_id = Some(media_id.clone());
        snapshot.playback.status = PlaybackStatus::Playing;
        snapshot.playback.display_assignments = vec![DisplayAssignment {
            target_id: "display:left".to_owned(),
            mode: DisplayMode::Independent,
            display_ids: vec!["left".to_owned()],
            wallpaper_id: media_id.clone(),
            status: PlaybackStatus::Playing,
            muted: true,
            volume: 0,
            pause_reasons: Vec::new(),
        }];
        snapshot.displays = vec![crate::model::DisplayInfo {
            id: "left".to_owned(),
            name: "Left".to_owned(),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            primary: true,
            connected: true,
        }];
        let invalid_storage = root.join("storage-file");
        fs::write(&invalid_storage, b"not a directory").expect("create invalid storage root");
        let state = runtime_state(invalid_storage, snapshot.clone());

        let failures = [
            state.commit_muted(false),
            state.commit_target_muted("display:left", false),
            state.commit_volume(42),
            state.commit_scale_mode(crate::model::ScaleMode::Contain),
            state.commit_stop_change(
                |core| core.stop_target("display:left"),
                |player| player.stop_target("display:left"),
            ),
            state.commit_stop_change(
                |core| {
                    core.stop();
                    Ok(())
                },
                MpvPlayerManager::stop,
            ),
            state.commit_display_refresh(vec![crate::model::DisplayInfo {
                id: "left".to_owned(),
                name: "Left Updated".to_owned(),
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                primary: true,
                connected: true,
            }]),
        ];
        for failure in failures {
            assert_eq!(
                failure.expect_err("reject failed persistence").code,
                "storage_failed"
            );
            assert_eq!(state.core().expect("core state").snapshot(), &snapshot);
        }

        let play_failure = state
            .commit_play(&media_id)
            .expect_err("reject unavailable player binary");
        assert_eq!(play_failure.code, "mpv_missing");
        assert_eq!(state.core().expect("core state").snapshot(), &snapshot);
        assert!(source.is_file());
        fs::remove_dir_all(root).expect("clean command test directory");
    }

    #[test]
    fn player_restore_plan_excludes_error_and_non_running_targets() {
        let mut snapshot = AppSnapshot::default();
        let playing = assignment("playing", Vec::new());
        let mut failed = assignment("failed", Vec::new());
        failed.status = PlaybackStatus::Error;
        let missing = assignment("missing", Vec::new());
        snapshot.playback.active_id = Some(playing.wallpaper_id.clone());
        snapshot.playback.status = PlaybackStatus::Error;
        snapshot.playback.display_assignments = vec![playing, failed, missing];

        let plan = build_player_restore_plan(&snapshot, |target_id| target_id != "missing");

        assert_eq!(plan.target_ids, vec!["playing"]);
        assert!(!plan.legacy);

        snapshot.playback.display_assignments.clear();
        let legacy = build_player_restore_plan(&snapshot, |_| true);
        assert!(!legacy.legacy);
    }

    #[test]
    fn project_homepage_points_to_the_public_repository() {
        assert_eq!(PROJECT_HOMEPAGE, "https://github.com/NiceBlueChai/wall");
    }

    fn runtime_state(root: PathBuf, snapshot: AppSnapshot) -> RuntimeState {
        RuntimeState {
            core: Mutex::new(WallCore::new(snapshot)),
            commit_gate: Mutex::new(()),
            player: Mutex::new(MpvPlayerManager::default()),
            publisher: SnapshotPublisher::spawn(|_| {}).expect("start snapshot publisher"),
            storage: Storage::new(root.clone()),
            mpv_binary: PathBuf::from("mpv.exe"),
            data_dir: root,
        }
    }

    fn snapshot_with_media(root: &PathBuf) -> (AppSnapshot, String, PathBuf) {
        let _ = fs::remove_dir_all(root);
        fs::create_dir_all(root).expect("create command test directory");
        let source = root.join("wallpaper.mp4");
        fs::write(&source, b"wallpaper").expect("create source media");
        let mut snapshot = AppSnapshot::default();
        let imported = import_media(&mut snapshot, std::slice::from_ref(&source))
            .expect("import source media");
        (snapshot, imported[0].id.clone(), source)
    }

    fn command_test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("wall-command-{label}-{}", std::process::id()))
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
