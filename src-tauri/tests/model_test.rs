//! 验证 Wall 的跨前后端状态契约。

use wall_lib::model::{
    AntiAliasing, AppSettings, AspectRatio, DisplayMode, PauseReason, PlaybackState, ScaleMode,
};

#[test]
fn settings_use_safe_offline_defaults() {
    let settings = AppSettings::default();

    assert!(!settings.auto_start);
    assert!(settings.close_to_tray);
    assert!(settings.restore_last_wallpaper);
    assert_eq!(settings.scale_mode, ScaleMode::Cover);
    assert_eq!(settings.frame_rate, 60);
    assert_eq!(settings.aspect_ratio, AspectRatio::Original);
    assert_eq!(settings.anti_aliasing, AntiAliasing::Balanced);
    assert!(settings.hardware_decoding);
    assert!(settings.default_muted);
    assert_eq!(settings.volume, 0);
    assert!(settings.pause_on_maximized);
    assert_eq!(settings.display_mode, DisplayMode::Independent);
    assert!(settings.selected_display_ids.is_empty());
}

#[test]
fn automatic_resume_keeps_manual_pause() {
    let mut playback = PlaybackState::default();
    playback.set_pause(PauseReason::Manual, true);
    playback.set_pause(PauseReason::Fullscreen, true);
    playback.set_pause(PauseReason::Fullscreen, false);

    assert!(playback.is_paused());
    assert_eq!(playback.pause_reasons, vec![PauseReason::Manual]);
}

#[test]
fn scale_modes_map_to_mpv_arguments() {
    assert_eq!(
        ScaleMode::Cover.mpv_arguments(),
        ["--keepaspect=yes", "--panscan=1.0"]
    );
    assert_eq!(
        ScaleMode::Contain.mpv_arguments(),
        ["--keepaspect=yes", "--panscan=0.0"]
    );
    assert_eq!(
        ScaleMode::Stretch.mpv_arguments(),
        ["--keepaspect=no", "--panscan=0.0"]
    );
}

#[test]
fn snapshot_serializes_category_contract() {
    let snapshot = serde_json::to_value(wall_lib::model::AppSnapshot::default())
        .expect("serialize default snapshot");

    assert_eq!(snapshot["categories"], serde_json::json!([]));
    assert_eq!(
        serde_json::to_value(sample_wallpaper()).expect("serialize wallpaper")["categoryIds"],
        serde_json::json!([])
    );
}

fn sample_wallpaper() -> wall_lib::model::WallpaperItem {
    wall_lib::model::WallpaperItem {
        id: "ocean".to_owned(),
        name: "Ocean Loop".to_owned(),
        path: r"D:\Wallpapers\ocean.mp4".to_owned(),
        kind: wall_lib::model::MediaKind::Video,
        format: "MP4".to_owned(),
        width: Some(1920),
        height: Some(1080),
        duration_seconds: Some(30.0),
        thumbnail_path: None,
        missing: false,
        category_ids: Vec::new(),
        settings: Default::default(),
    }
}
