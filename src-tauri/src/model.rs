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

/// 控制视频内容在缩放前使用的逻辑画幅。
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AspectRatio {
    #[default]
    Original,
    Screen,
    Ratio16x9,
    Ratio16x10,
    Ratio21x9,
    Ratio32x9,
    Ratio4x3,
    Ratio1x1,
    Ratio9x16,
}

impl AspectRatio {
    /// 返回 mpv 的画幅覆盖值；Screen 使用目标显示区域尺寸。
    pub fn mpv_value(self, screen_width: i32, screen_height: i32) -> String {
        match self {
            Self::Original => "-1".to_owned(),
            Self::Screen => format!("{screen_width}:{screen_height}"),
            Self::Ratio16x9 => "16:9".to_owned(),
            Self::Ratio16x10 => "16:10".to_owned(),
            Self::Ratio21x9 => "21:9".to_owned(),
            Self::Ratio32x9 => "32:9".to_owned(),
            Self::Ratio4x3 => "4:3".to_owned(),
            Self::Ratio1x1 => "1:1".to_owned(),
            Self::Ratio9x16 => "9:16".to_owned(),
        }
    }
}

/// 控制 mpv 视频缩放器质量。
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AntiAliasing {
    Off,
    #[default]
    Balanced,
    High,
}

impl AntiAliasing {
    /// 返回 mpv 对应的缩放器名称。
    pub fn mpv_scale(self) -> &'static str {
        match self {
            Self::Off => "bilinear",
            Self::Balanced => "spline36",
            Self::High => "ewa_lanczossharp",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PauseReason {
    Manual,
    Fullscreen,
    Maximized,
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

/// 控制所选显示器之间的壁纸布局方式。
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplayMode {
    #[default]
    Independent,
    Clone,
    Span,
}

/// 描述 Windows 虚拟桌面中的一个显示器矩形。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
    pub connected: bool,
}

/// 记录一个独立屏幕或联动屏幕组当前运行的壁纸。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayAssignment {
    pub target_id: String,
    pub mode: DisplayMode,
    pub display_ids: Vec<String>,
    pub wallpaper_id: String,
    pub status: PlaybackStatus,
    pub muted: bool,
    pub volume: u8,
    #[serde(default)]
    pub pause_reasons: Vec<PauseReason>,
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
    #[serde(default)]
    pub display_assignments: Vec<DisplayAssignment>,
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
            display_assignments: Vec::new(),
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
    #[serde(default)]
    pub aspect_ratio: AspectRatio,
    #[serde(default)]
    pub anti_aliasing: AntiAliasing,
    pub hardware_decoding: bool,
    pub default_muted: bool,
    pub volume: u8,
    pub pause_on_fullscreen: bool,
    #[serde(default = "default_true")]
    pub pause_on_maximized: bool,
    pub pause_on_battery: bool,
    pub pause_on_display_sleep: bool,
    #[serde(default)]
    pub display_mode: DisplayMode,
    #[serde(default)]
    pub selected_display_ids: Vec<String>,
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
            aspect_ratio: AspectRatio::Original,
            anti_aliasing: AntiAliasing::Balanced,
            hardware_decoding: true,
            default_muted: true,
            volume: 0,
            pause_on_fullscreen: true,
            pause_on_maximized: true,
            pause_on_battery: true,
            pause_on_display_sleep: true,
            display_mode: DisplayMode::Independent,
            selected_display_ids: Vec::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Video,
    Image,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
}

/// 保存单张壁纸相对全局设置的可选覆盖项。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_mode: Option<ScaleMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<AspectRatio>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anti_aliasing: Option<AntiAliasing>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_rate: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardware_decoding: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub muted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<u8>,
}

impl WallpaperSettings {
    /// 将当前存在的覆盖项合并到一份全局设置副本。
    pub fn apply_to(&self, settings: &mut AppSettings) {
        if let Some(value) = self.scale_mode {
            settings.scale_mode = value;
        }
        if let Some(value) = self.aspect_ratio {
            settings.aspect_ratio = value;
        }
        if let Some(value) = self.anti_aliasing {
            settings.anti_aliasing = value;
        }
        if let Some(value) = self.frame_rate {
            settings.frame_rate = value;
        }
        if let Some(value) = self.hardware_decoding {
            settings.hardware_decoding = value;
        }
        if let Some(value) = self.muted {
            settings.default_muted = value;
        }
        if let Some(value) = self.volume {
            settings.volume = value;
        }
    }
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
    #[serde(default)]
    pub category_ids: Vec<String>,
    #[serde(default)]
    pub settings: WallpaperSettings,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub library: Vec<WallpaperItem>,
    #[serde(default)]
    pub categories: Vec<Category>,
    pub settings: AppSettings,
    pub playback: PlaybackState,
    #[serde(default)]
    pub displays: Vec<DisplayInfo>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}
