//! 提供所有界面入口共享的业务状态机。

use crate::media::{classify_media, import_media};
use crate::model::{
    AppError, AppSettings, AppSnapshot, PauseReason, PlaybackStatus, ScaleMode, WallpaperItem,
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

    /// 将媒体设置为当前壁纸并重置手动暂停状态。
    pub fn play(&mut self, id: &str) -> Result<(), AppError> {
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
        self.snapshot.playback.muted = self.snapshot.settings.default_muted;
        self.snapshot.playback.volume = self.snapshot.settings.volume;
        self.snapshot.playback.last_error = None;
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
        Ok(())
    }

    /// 更新全屏、电池或休眠等暂停来源。
    pub fn set_pause_reason(&mut self, reason: PauseReason, paused: bool) {
        self.snapshot.playback.set_pause(reason, paused);
    }

    /// 停止当前壁纸并清除全部暂停来源。
    pub fn stop(&mut self) {
        self.snapshot.playback.active_id = None;
        self.snapshot.playback.status = PlaybackStatus::Idle;
        self.snapshot.playback.pause_reasons.clear();
        self.snapshot.playback.last_error = None;
    }

    /// 设置当前播放静音状态。
    pub fn set_muted(&mut self, muted: bool) {
        self.snapshot.playback.muted = muted;
    }

    /// 设置 0 到 100 的壁纸音量。
    pub fn set_volume(&mut self, volume: u8) -> Result<(), AppError> {
        if volume > 100 {
            return Err(error("invalid_volume", "音量必须在 0 到 100 之间"));
        }
        self.snapshot.playback.volume = volume;
        self.snapshot.settings.volume = volume;
        Ok(())
    }

    /// 设置所有媒体共用的缩放模式。
    pub fn set_scale_mode(&mut self, mode: ScaleMode) {
        self.snapshot.settings.scale_mode = mode;
    }

    /// 替换并验证全部用户设置。
    pub fn update_settings(&mut self, settings: AppSettings) -> Result<(), AppError> {
        if settings.volume > 100 || ![30, 60].contains(&settings.frame_rate) {
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
        if self.snapshot.playback.active_id.as_deref() == Some(id) {
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
    }
}

fn error(code: &str, message: &str) -> AppError {
    AppError {
        code: code.to_owned(),
        message: message.to_owned(),
        recoverable: true,
    }
}
