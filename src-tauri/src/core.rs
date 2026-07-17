//! 提供所有界面入口共享的业务状态机。

use crate::media::{classify_media, import_media};
use crate::model::{
    AppError, AppSettings, AppSnapshot, Category, DisplayAssignment, DisplayInfo, DisplayMode,
    PauseReason, PlaybackStatus, ScaleMode, WallpaperItem, WallpaperSettings,
};
use std::path::{Path, PathBuf};

pub struct WallCore {
    snapshot: AppSnapshot,
}

impl WallCore {
    /// 使用已恢复的本地快照创建业务状态机。
    pub fn new(snapshot: AppSnapshot) -> Self {
        Self { snapshot }
    }

    /// 返回可序列化给 Vue 和托盘的完整状态。
    pub fn snapshot(&self) -> &AppSnapshot {
        &self.snapshot
    }

    /// 导入通过原生文件对话框选择的媒体路径。
    pub fn import_paths(&mut self, paths: &[PathBuf]) -> Result<Vec<WallpaperItem>, AppError> {
        import_media(&mut self.snapshot, paths)
    }

    /// 创建名称唯一的用户分类，并返回持久化后的分类。
    pub fn create_category(&mut self, name: &str) -> Result<Category, AppError> {
        let name = normalized_category_name(name)?;
        if self.category_name_exists(&name, None) {
            return Err(error("category_exists", "已存在同名分类"));
        }
        let category = Category {
            id: uuid::Uuid::new_v4().to_string(),
            name,
        };
        self.snapshot.categories.push(category.clone());
        Ok(category)
    }

    /// 重命名用户分类，保持分类标识和壁纸归属不变。
    pub fn rename_category(&mut self, id: &str, name: &str) -> Result<(), AppError> {
        let name = normalized_category_name(name)?;
        if self.category_name_exists(&name, Some(id)) {
            return Err(error("category_exists", "已存在同名分类"));
        }
        let category = self
            .snapshot
            .categories
            .iter_mut()
            .find(|category| category.id == id)
            .ok_or_else(|| error("category_not_found", "分类不存在"))?;
        category.name = name;
        Ok(())
    }

    /// 删除分类并解除全部壁纸归属，不删除任何媒体项目或源文件。
    pub fn delete_category(&mut self, id: &str) -> Result<(), AppError> {
        let length = self.snapshot.categories.len();
        self.snapshot
            .categories
            .retain(|category| category.id != id);
        if self.snapshot.categories.len() == length {
            return Err(error("category_not_found", "分类不存在"));
        }
        for item in &mut self.snapshot.library {
            item.category_ids.retain(|category_id| category_id != id);
        }
        Ok(())
    }

    /// 原子地为一组壁纸添加或移除指定分类。
    pub fn set_category_membership(
        &mut self,
        media_ids: &[String],
        category_id: &str,
        assigned: bool,
    ) -> Result<(), AppError> {
        if !self
            .snapshot
            .categories
            .iter()
            .any(|category| category.id == category_id)
        {
            return Err(error("category_not_found", "分类不存在"));
        }
        if media_ids
            .iter()
            .any(|id| !self.snapshot.library.iter().any(|item| &item.id == id))
        {
            return Err(error("wallpaper_not_found", "壁纸不在媒体库中"));
        }
        for item in self
            .snapshot
            .library
            .iter_mut()
            .filter(|item| media_ids.contains(&item.id))
        {
            if assigned && !item.category_ids.iter().any(|id| id == category_id) {
                item.category_ids.push(category_id.to_owned());
            } else if !assigned {
                item.category_ids.retain(|id| id != category_id);
            }
        }
        Ok(())
    }

    /// 用当前系统枚举结果刷新显示器列表，并为首次运行选择主显示器。
    pub fn set_displays(&mut self, mut displays: Vec<DisplayInfo>) {
        for previous in &self.snapshot.displays {
            if !displays.iter().any(|display| display.id == previous.id) {
                let mut offline = previous.clone();
                offline.connected = false;
                offline.primary = false;
                displays.push(offline);
            }
        }
        for selected_id in &self.snapshot.settings.selected_display_ids {
            if !displays.iter().any(|display| display.id == *selected_id) {
                displays.push(DisplayInfo {
                    id: selected_id.clone(),
                    name: selected_id.clone(),
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 0,
                    primary: false,
                    connected: false,
                });
            }
        }
        self.snapshot.displays = displays;
        if self.snapshot.settings.selected_display_ids.is_empty()
            && let Some(display) = self
                .snapshot
                .displays
                .iter()
                .find(|display| display.primary)
                .or_else(|| self.snapshot.displays.first())
        {
            self.snapshot.settings.selected_display_ids = vec![display.id.clone()];
            self.snapshot.settings.display_mode = DisplayMode::Independent;
        }
    }

    /// 验证并选择独立、复制或铺展显示器布局。
    pub fn set_display_layout(
        &mut self,
        mode: DisplayMode,
        display_ids: Vec<String>,
    ) -> Result<(), AppError> {
        let unique = display_ids.iter().collect::<std::collections::HashSet<_>>();
        let required = if mode == DisplayMode::Independent {
            1
        } else {
            2
        };
        let valid_count = display_ids.len() == required || (required == 2 && display_ids.len() > 2);
        let all_connected = display_ids.iter().all(|id| {
            self.snapshot
                .displays
                .iter()
                .any(|display| display.id == *id && display.connected)
        });
        if !valid_count || unique.len() != display_ids.len() || !all_connected {
            return Err(error(
                "invalid_display_selection",
                "显示器选择不符合当前播放模式",
            ));
        }
        self.snapshot.settings.display_mode = mode;
        self.snapshot.settings.selected_display_ids = display_ids;
        Ok(())
    }

    /// 替换单张壁纸的覆盖设置；空结构表示恢复全局设置。
    pub fn update_wallpaper_settings(
        &mut self,
        id: &str,
        settings: WallpaperSettings,
    ) -> Result<(), AppError> {
        validate_wallpaper_settings(&settings)?;
        let item = self
            .snapshot
            .library
            .iter_mut()
            .find(|item| item.id == id)
            .ok_or_else(|| error("wallpaper_not_found", "壁纸不在媒体库中"))?;
        item.settings = settings;
        Ok(())
    }

    /// 合并全局设置与指定壁纸覆盖项，返回实际播放器配置。
    pub fn effective_settings(&self, id: &str) -> Result<AppSettings, AppError> {
        let item = self
            .snapshot
            .library
            .iter()
            .find(|item| item.id == id)
            .ok_or_else(|| error("wallpaper_not_found", "壁纸不在媒体库中"))?;
        let mut settings = self.snapshot.settings.clone();
        item.settings.apply_to(&mut settings);
        Ok(settings)
    }

    /// 将媒体设置为当前壁纸并重置手动暂停状态。
    pub fn play(&mut self, id: &str) -> Result<(), AppError> {
        let settings = self.effective_settings(id)?;
        let item = self
            .snapshot
            .library
            .iter_mut()
            .find(|item| item.id == id)
            .ok_or_else(|| error("wallpaper_not_found", "壁纸不在媒体库中"))?;
        if !Path::new(&item.path).is_file() {
            item.missing = true;
            return Err(error("file_missing", "壁纸文件已被移动或删除"));
        }
        item.missing = false;
        self.snapshot.playback.active_id = Some(id.to_owned());
        self.snapshot.playback.pause_reasons.clear();
        self.snapshot.playback.status = PlaybackStatus::Playing;
        self.snapshot.playback.muted = settings.default_muted;
        self.snapshot.playback.volume = settings.volume;
        self.snapshot.playback.last_error = None;
        self.record_display_assignment(id);
        Ok(())
    }

    /// 切换用户手动暂停原因，不影响自动暂停来源。
    pub fn toggle_pause(&mut self) -> Result<(), AppError> {
        if self.snapshot.playback.active_id.is_none() {
            return Err(error("no_wallpaper", "当前没有正在运行的壁纸"));
        }
        let paused = !self
            .snapshot
            .playback
            .pause_reasons
            .contains(&PauseReason::Manual);
        self.snapshot
            .playback
            .set_pause(PauseReason::Manual, paused);
        if !self.snapshot.playback.display_assignments.is_empty() {
            for assignment in &mut self.snapshot.playback.display_assignments {
                if paused && !assignment.pause_reasons.contains(&PauseReason::Manual) {
                    assignment.pause_reasons.push(PauseReason::Manual);
                } else if !paused {
                    assignment
                        .pause_reasons
                        .retain(|reason| *reason != PauseReason::Manual);
                }
                assignment.status = if assignment.pause_reasons.is_empty() {
                    PlaybackStatus::Playing
                } else {
                    PlaybackStatus::Paused
                };
            }
            self.refresh_aggregate_playback();
        }
        Ok(())
    }

    /// 切换单个显示目标的手动暂停状态。
    pub fn toggle_target_pause(&mut self, target_id: &str) -> Result<(), AppError> {
        let assignment = self
            .snapshot
            .playback
            .display_assignments
            .iter_mut()
            .find(|assignment| assignment.target_id == target_id)
            .ok_or_else(|| error("display_target_not_found", "显示目标不存在"))?;
        let paused = !assignment.pause_reasons.contains(&PauseReason::Manual);
        if paused {
            assignment.pause_reasons.push(PauseReason::Manual);
            assignment.status = PlaybackStatus::Paused;
        } else {
            assignment
                .pause_reasons
                .retain(|reason| *reason != PauseReason::Manual);
            assignment.status = if assignment.pause_reasons.is_empty() {
                PlaybackStatus::Playing
            } else {
                PlaybackStatus::Paused
            };
        }
        self.refresh_aggregate_playback();
        Ok(())
    }

    /// 添加或移除单个显示目标的自动暂停原因。
    pub fn set_target_pause_reason(
        &mut self,
        target_id: &str,
        reason: PauseReason,
        paused: bool,
    ) -> Result<(), AppError> {
        let assignment = self
            .snapshot
            .playback
            .display_assignments
            .iter_mut()
            .find(|assignment| assignment.target_id == target_id)
            .ok_or_else(|| error("display_target_not_found", "显示目标不存在"))?;
        if paused && !assignment.pause_reasons.contains(&reason) {
            assignment.pause_reasons.push(reason);
        } else if !paused {
            assignment
                .pause_reasons
                .retain(|existing| *existing != reason);
        }
        assignment.status = if assignment.pause_reasons.is_empty() {
            PlaybackStatus::Playing
        } else {
            PlaybackStatus::Paused
        };
        self.refresh_aggregate_playback();
        Ok(())
    }

    /// 设置单个显示目标的静音状态。
    pub fn set_target_muted(&mut self, target_id: &str, muted: bool) -> Result<(), AppError> {
        let assignment = self
            .snapshot
            .playback
            .display_assignments
            .iter_mut()
            .find(|assignment| assignment.target_id == target_id)
            .ok_or_else(|| error("display_target_not_found", "显示目标不存在"))?;
        assignment.muted = muted;
        self.snapshot.playback.muted = self
            .snapshot
            .playback
            .display_assignments
            .iter()
            .all(|assignment| assignment.muted);
        Ok(())
    }

    /// 将单个显示目标标记为播放错误并保留其分配以便后台重试。
    pub fn set_target_error(&mut self, target_id: &str, message: String) -> Result<(), AppError> {
        let assignment = self
            .snapshot
            .playback
            .display_assignments
            .iter_mut()
            .find(|assignment| assignment.target_id == target_id)
            .ok_or_else(|| error("display_target_not_found", "显示目标不存在"))?;
        assignment.status = PlaybackStatus::Error;
        self.snapshot.playback.last_error = Some(message);
        self.refresh_aggregate_playback();
        Ok(())
    }

    /// 根据暂停原因恢复单个显示目标的运行状态。
    pub fn restore_target_status(&mut self, target_id: &str) -> Result<(), AppError> {
        let assignment = self
            .snapshot
            .playback
            .display_assignments
            .iter_mut()
            .find(|assignment| assignment.target_id == target_id)
            .ok_or_else(|| error("display_target_not_found", "显示目标不存在"))?;
        assignment.status = if assignment.pause_reasons.is_empty() {
            PlaybackStatus::Playing
        } else {
            PlaybackStatus::Paused
        };
        if !self
            .snapshot
            .playback
            .display_assignments
            .iter()
            .any(|assignment| assignment.status == PlaybackStatus::Error)
        {
            self.snapshot.playback.last_error = None;
        }
        self.refresh_aggregate_playback();
        Ok(())
    }

    /// 更新使用指定壁纸的显示目标音频状态，不影响其他壁纸和暂停原因。
    pub fn set_media_playback_settings(
        &mut self,
        media_id: &str,
        muted: bool,
        volume: u8,
    ) -> Result<(), AppError> {
        if volume > 100 {
            return Err(error("invalid_volume", "音量必须在 0 到 100 之间"));
        }
        if !self.snapshot.library.iter().any(|item| item.id == media_id) {
            return Err(error("wallpaper_not_found", "壁纸不在媒体库中"));
        }
        let mut matched = false;
        for assignment in &mut self.snapshot.playback.display_assignments {
            if assignment.wallpaper_id == media_id {
                assignment.muted = muted;
                assignment.volume = volume;
                matched = true;
            }
        }
        if matched {
            self.refresh_aggregate_playback();
        } else if self.snapshot.playback.active_id.as_deref() == Some(media_id) {
            self.snapshot.playback.muted = muted;
            self.snapshot.playback.volume = volume;
        }
        Ok(())
    }

    /// 停止单个显示目标并保留其他独立目标。
    pub fn stop_target(&mut self, target_id: &str) -> Result<(), AppError> {
        let length = self.snapshot.playback.display_assignments.len();
        self.snapshot
            .playback
            .display_assignments
            .retain(|assignment| assignment.target_id != target_id);
        if self.snapshot.playback.display_assignments.len() == length {
            return Err(error("display_target_not_found", "显示目标不存在"));
        }
        self.refresh_aggregate_playback();
        Ok(())
    }

    /// 更新全屏、电池或休眠等暂停来源。
    pub fn set_pause_reason(&mut self, reason: PauseReason, paused: bool) {
        self.snapshot.playback.set_pause(reason, paused);
        if !self.snapshot.playback.display_assignments.is_empty() {
            for assignment in &mut self.snapshot.playback.display_assignments {
                if paused && !assignment.pause_reasons.contains(&reason) {
                    assignment.pause_reasons.push(reason);
                } else if !paused {
                    assignment
                        .pause_reasons
                        .retain(|existing| *existing != reason);
                }
                assignment.status = if assignment.pause_reasons.is_empty() {
                    PlaybackStatus::Playing
                } else {
                    PlaybackStatus::Paused
                };
            }
            self.refresh_aggregate_playback();
        }
    }

    /// 停止当前壁纸并清除全部暂停来源。
    pub fn stop(&mut self) {
        self.snapshot.playback.active_id = None;
        self.snapshot.playback.status = PlaybackStatus::Idle;
        self.snapshot.playback.pause_reasons.clear();
        self.snapshot.playback.last_error = None;
        self.snapshot.playback.display_assignments.clear();
    }

    /// 设置当前播放静音状态。
    pub fn set_muted(&mut self, muted: bool) {
        self.snapshot.playback.muted = muted;
        for assignment in &mut self.snapshot.playback.display_assignments {
            assignment.muted = muted;
        }
    }

    /// 设置 0 到 100 的壁纸音量。
    pub fn set_volume(&mut self, volume: u8) -> Result<(), AppError> {
        if volume > 100 {
            return Err(error("invalid_volume", "音量必须在 0 到 100 之间"));
        }
        self.snapshot.playback.volume = volume;
        self.snapshot.settings.volume = volume;
        for assignment in &mut self.snapshot.playback.display_assignments {
            assignment.volume = volume;
        }
        Ok(())
    }

    /// 设置所有媒体共用的缩放模式。
    pub fn set_scale_mode(&mut self, mode: ScaleMode) {
        self.snapshot.settings.scale_mode = mode;
    }

    /// 替换并验证全部用户设置。
    pub fn update_settings(&mut self, settings: AppSettings) -> Result<(), AppError> {
        if settings.volume > 100 || ![0, 24, 30, 60].contains(&settings.frame_rate) {
            return Err(error("invalid_settings", "音量或帧率设置无效"));
        }
        self.snapshot.settings = settings;
        Ok(())
    }

    /// 从媒体库移除项目；移除当前项目时同时停止播放。
    pub fn remove(&mut self, id: &str) -> Result<(), AppError> {
        let length = self.snapshot.library.len();
        self.snapshot.library.retain(|item| item.id != id);
        if self.snapshot.library.len() == length {
            return Err(error("wallpaper_not_found", "壁纸不在媒体库中"));
        }
        let had_assignments = !self.snapshot.playback.display_assignments.is_empty();
        self.snapshot
            .playback
            .display_assignments
            .retain(|assignment| assignment.wallpaper_id != id);
        if had_assignments {
            self.refresh_aggregate_playback();
        } else if self.snapshot.playback.active_id.as_deref() == Some(id) {
            self.stop();
        }
        Ok(())
    }

    /// 将媒体库项目重新定位到同类的本地文件，同时保留项目标识。
    pub fn relocate(&mut self, id: &str, new_path: &Path) -> Result<(), AppError> {
        let canonical = new_path
            .canonicalize()
            .map_err(|_| error("file_missing", "新文件不存在或无法访问"))?;
        let kind = classify_media(&canonical)
            .ok_or_else(|| error("unsupported_format", "新文件格式不受支持"))?;
        let item = self
            .snapshot
            .library
            .iter_mut()
            .find(|item| item.id == id)
            .ok_or_else(|| error("wallpaper_not_found", "壁纸不在媒体库中"))?;
        if item.kind != kind {
            return Err(error(
                "media_kind_mismatch",
                "视频只能重新定位到视频，图片只能重新定位到图片",
            ));
        }
        item.path = canonical.to_string_lossy().into_owned();
        item.name = canonical
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("未命名壁纸")
            .to_owned();
        item.format = canonical
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_uppercase();
        item.missing = false;
        Ok(())
    }

    /// 返回指定壁纸的本地路径。
    pub fn media_path(&self, id: &str) -> Result<PathBuf, AppError> {
        self.snapshot
            .library
            .iter()
            .find(|item| item.id == id)
            .map(|item| PathBuf::from(&item.path))
            .ok_or_else(|| error("wallpaper_not_found", "壁纸不在媒体库中"))
    }

    /// 记录播放器错误并向界面切换到错误状态。
    pub fn set_playback_error(&mut self, message: String) {
        self.snapshot.playback.status = PlaybackStatus::Error;
        self.snapshot.playback.last_error = Some(message);
        self.sync_assignment_status();
    }

    fn category_name_exists(&self, name: &str, excluded_id: Option<&str>) -> bool {
        self.snapshot.categories.iter().any(|category| {
            Some(category.id.as_str()) != excluded_id && category.name.eq_ignore_ascii_case(name)
        })
    }

    fn record_display_assignment(&mut self, wallpaper_id: &str) {
        let mut display_ids = self.snapshot.settings.selected_display_ids.clone();
        if display_ids.is_empty() {
            return;
        }
        display_ids.sort();
        let mode = self.snapshot.settings.display_mode;
        let prefix = match mode {
            DisplayMode::Independent => "display",
            DisplayMode::Clone => "clone",
            DisplayMode::Span => "span",
        };
        let target_id = format!("{prefix}:{}", display_ids.join("+"));
        let assignment = DisplayAssignment {
            target_id: target_id.clone(),
            mode,
            display_ids,
            wallpaper_id: wallpaper_id.to_owned(),
            status: self.snapshot.playback.status,
            muted: self.snapshot.playback.muted,
            volume: self.snapshot.playback.volume,
            pause_reasons: self.snapshot.playback.pause_reasons.clone(),
        };
        self.snapshot
            .playback
            .display_assignments
            .retain(|existing| !shares_display(existing, &assignment));
        self.snapshot.playback.display_assignments.push(assignment);
    }

    fn sync_assignment_status(&mut self) {
        for assignment in &mut self.snapshot.playback.display_assignments {
            assignment.status = self.snapshot.playback.status;
        }
    }

    fn refresh_aggregate_playback(&mut self) {
        let playback = &mut self.snapshot.playback;
        if let Some(last) = playback.display_assignments.last() {
            playback.active_id = Some(last.wallpaper_id.clone());
            playback.status = if playback
                .display_assignments
                .iter()
                .any(|assignment| assignment.status == PlaybackStatus::Error)
            {
                PlaybackStatus::Error
            } else if playback
                .display_assignments
                .iter()
                .all(|assignment| assignment.status == PlaybackStatus::Paused)
            {
                PlaybackStatus::Paused
            } else {
                PlaybackStatus::Playing
            };
            playback.muted = playback
                .display_assignments
                .iter()
                .all(|assignment| assignment.muted);
            playback.volume = last.volume;
        } else {
            playback.active_id = None;
            playback.status = PlaybackStatus::Idle;
            playback.pause_reasons.clear();
        }
    }
}

fn shares_display(left: &DisplayAssignment, right: &DisplayAssignment) -> bool {
    left.display_ids
        .iter()
        .any(|display_id| right.display_ids.contains(display_id))
}

fn normalized_category_name(name: &str) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(error("invalid_category_name", "分类名称不能为空"));
    }
    Ok(name.to_owned())
}

fn validate_wallpaper_settings(settings: &WallpaperSettings) -> Result<(), AppError> {
    if settings.volume.is_some_and(|volume| volume > 100)
        || settings
            .frame_rate
            .is_some_and(|frame_rate| ![0, 24, 30, 60].contains(&frame_rate))
    {
        return Err(error("invalid_settings", "音量或帧率设置无效"));
    }
    Ok(())
}

fn error(code: &str, message: &str) -> AppError {
    AppError {
        code: code.to_owned(),
        message: message.to_owned(),
        recoverable: true,
    }
}
