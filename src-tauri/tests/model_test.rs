//! 验证 Wall 的跨前后端状态契约。

use wall_lib::model::{AppSettings, PauseReason, PlaybackState, ScaleMode};

#[test]
fn settings_use_safe_offline_defaults() {
    let settings = AppSettings::default();

    assert!(!settings.auto_start);
    assert!(settings.close_to_tray);
    assert!(settings.restore_last_wallpaper);
    assert_eq!(settings.scale_mode, ScaleMode::Cover);
    assert_eq!(settings.frame_rate, 60);
    assert!(settings.hardware_decoding);
    assert!(settings.default_muted);
    assert_eq!(settings.volume, 0);
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
