//! 定义前端、托盘和播放器共享的状态契约。

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScaleMode {
    #[default]
    Cover,
    Contain,
    Stretch,
}

impl ScaleMode {
    /// 返回与缩放模式对应的最小 mpv 参数集合。
    pub fn mpv_arguments(self) -> [&'static str; 2] {
        match self {
            Self::Cover => ["--keepaspect=yes", "--panscan=1.0"],
            Self::Contain => ["--keepaspect=yes", "--panscan=0.0"],
            Self::Stretch => ["--keepaspect=no", "--panscan=0.0"],
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PauseReason {
    Manual,
    Fullscreen,
    Battery,
    DisplaySleep,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackStatus {
    #[default]
    Idle,
    Playing,
    Paused,
    Error,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackState {
    pub active_id: Option<String>,
    pub status: PlaybackStatus,
    pub muted: bool,
    pub volume: u8,
    pub pause_reasons: Vec<PauseReason>,
    pub last_error: Option<String>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            active_id: None,
            status: PlaybackStatus::Idle,
            muted: true,
            volume: 0,
            pause_reasons: Vec::new(),
            last_error: None,
        }
    }
}

impl PlaybackState {
    /// 添加或移除暂停原因，并保持其他暂停来源不变。
    pub fn set_pause(&mut self, reason: PauseReason, paused: bool) {
        if paused && !self.pause_reasons.contains(&reason) {
            self.pause_reasons.push(reason);
        } else if !paused {
            self.pause_reasons.retain(|existing| *existing != reason);
        }
        if self.active_id.is_some() {
            self.status = if self.pause_reasons.is_empty() {
                PlaybackStatus::Playing
            } else {
                PlaybackStatus::Paused
            };
        }
    }

    /// 判断播放器当前是否被任一来源暂停。
    pub fn is_paused(&self) -> bool {
        !self.pause_reasons.is_empty()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub auto_start: bool,
    pub close_to_tray: bool,
    pub restore_last_wallpaper: bool,
    pub language: String,
    pub scale_mode: ScaleMode,
    pub frame_rate: u8,
    pub hardware_decoding: bool,
    pub default_muted: bool,
    pub volume: u8,
    pub pause_on_fullscreen: bool,
    pub pause_on_battery: bool,
    pub pause_on_display_sleep: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_start: false,
            close_to_tray: true,
            restore_last_wallpaper: true,
            language: "zh-CN".to_owned(),
            scale_mode: ScaleMode::Cover,
            frame_rate: 60,
            hardware_decoding: true,
            default_muted: true,
            volume: 0,
            pause_on_fullscreen: true,
            pause_on_battery: true,
            pause_on_display_sleep: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Video,
    Image,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub kind: MediaKind,
    pub format: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_seconds: Option<f64>,
    pub thumbnail_path: Option<String>,
    pub missing: bool,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub library: Vec<WallpaperItem>,
    pub settings: AppSettings,
    pub playback: PlaybackState,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}
