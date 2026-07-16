//! 验证主界面、托盘和播放器共享的业务状态机。

use std::fs;
use wall_lib::core::WallCore;
use wall_lib::media::import_media;
use wall_lib::model::{AppSnapshot, PauseReason, PlaybackStatus, ScaleMode};

#[test]
fn play_pause_and_stop_share_one_state_machine() {
    let root = test_root("playback");
    let video = root.join("ocean.mp4");
    fs::write(&video, b"video").expect("create video");
    let mut snapshot = AppSnapshot::default();
    let id = import_media(&mut snapshot, &[video]).expect("import")[0]
        .id
        .clone();
    let mut core = WallCore::new(snapshot);

    core.play(&id).expect("play");
    assert_eq!(core.snapshot().playback.status, PlaybackStatus::Playing);
    core.toggle_pause().expect("pause");
    assert_eq!(
        core.snapshot().playback.pause_reasons,
        vec![PauseReason::Manual]
    );
    core.toggle_pause().expect("resume");
    assert_eq!(core.snapshot().playback.status, PlaybackStatus::Playing);
    core.stop();
    assert_eq!(core.snapshot().playback.status, PlaybackStatus::Idle);
    assert!(core.snapshot().playback.active_id.is_none());
    fs::remove_dir_all(root).expect("clean test directory");
}

#[test]
fn validates_volume_and_active_wallpaper() {
    let mut core = WallCore::new(AppSnapshot::default());

    assert_eq!(
        core.set_volume(101)
            .expect_err("reject invalid volume")
            .code,
        "invalid_volume"
    );
    assert_eq!(
        core.play("missing").expect_err("reject missing item").code,
        "wallpaper_not_found"
    );
}

#[test]
fn setting_scale_mode_updates_the_shared_snapshot() {
    let mut core = WallCore::new(AppSnapshot::default());

    core.set_scale_mode(ScaleMode::Contain);

    assert_eq!(core.snapshot().settings.scale_mode, ScaleMode::Contain);
}

#[test]
fn relocates_a_missing_wallpaper_without_changing_its_id() {
    let root = test_root("relocate");
    let original = root.join("old.mp4");
    let replacement = root.join("new.mp4");
    fs::write(&original, b"old").expect("create original");
    fs::write(&replacement, b"new").expect("create replacement");
    let mut snapshot = AppSnapshot::default();
    let id = import_media(&mut snapshot, &[original.clone()]).expect("import")[0]
        .id
        .clone();
    fs::remove_file(original).expect("remove original");
    let mut core = WallCore::new(snapshot);

    core.relocate(&id, &replacement).expect("relocate");

    assert_eq!(core.snapshot().library[0].id, id);
    assert!(core.snapshot().library[0].path.ends_with("new.mp4"));
    assert!(!core.snapshot().library[0].missing);
    fs::remove_dir_all(root).expect("clean test directory");
}

fn test_root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("wall-core-{name}-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove stale test directory");
    }
    fs::create_dir_all(&root).expect("create test directory");
    root
}
