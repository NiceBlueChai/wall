//! 验证主界面、托盘和播放器共享的业务状态机。

use std::fs;
use wall_lib::core::WallCore;
use wall_lib::media::import_media;
use wall_lib::model::{
    AntiAliasing, AppSnapshot, AspectRatio, DisplayInfo, DisplayMode, PauseReason, PlaybackStatus,
    ScaleMode, WallpaperSettings,
};

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
    let id = import_media(&mut snapshot, std::slice::from_ref(&original)).expect("import")[0]
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

#[test]
fn category_lifecycle_preserves_media_and_updates_assignments() {
    let root = test_root("categories");
    let video = root.join("ocean.mp4");
    fs::write(&video, b"video").expect("create video");
    let mut snapshot = AppSnapshot::default();
    let media_id = import_media(&mut snapshot, &[video]).expect("import")[0]
        .id
        .clone();
    let mut core = WallCore::new(snapshot);

    let category = core
        .create_category("  自然风景  ")
        .expect("create category");
    assert_eq!(category.name, "自然风景");
    assert_eq!(
        core.create_category("自然风景")
            .expect_err("reject duplicate category")
            .code,
        "category_exists"
    );

    core.set_category_membership(std::slice::from_ref(&media_id), &category.id, true)
        .expect("assign category");
    assert_eq!(
        core.snapshot().library[0].category_ids,
        vec![category.id.clone()]
    );

    core.rename_category(&category.id, "森林")
        .expect("rename category");
    assert_eq!(core.snapshot().categories[0].name, "森林");

    core.delete_category(&category.id).expect("delete category");
    assert_eq!(core.snapshot().library.len(), 1);
    assert!(core.snapshot().library[0].category_ids.is_empty());
    fs::remove_dir_all(root).expect("clean test directory");
}

#[test]
fn category_membership_rejects_unknown_ids_without_partial_updates() {
    let root = test_root("category-atomic");
    let video = root.join("ocean.mp4");
    fs::write(&video, b"video").expect("create video");
    let mut snapshot = AppSnapshot::default();
    let media_id = import_media(&mut snapshot, &[video]).expect("import")[0]
        .id
        .clone();
    let mut core = WallCore::new(snapshot);
    let category = core.create_category("自然风景").expect("create category");

    let error = core
        .set_category_membership(&[media_id, "missing".to_owned()], &category.id, true)
        .expect_err("reject unknown media");

    assert_eq!(error.code, "wallpaper_not_found");
    assert!(core.snapshot().library[0].category_ids.is_empty());
    fs::remove_dir_all(root).expect("clean test directory");
}

#[test]
fn wallpaper_settings_override_and_restore_global_defaults() {
    let root = test_root("wallpaper-settings");
    let video = root.join("ocean.mp4");
    fs::write(&video, b"video").expect("create video");
    let mut snapshot = AppSnapshot::default();
    let media_id = import_media(&mut snapshot, &[video]).expect("import")[0]
        .id
        .clone();
    let mut core = WallCore::new(snapshot);
    let overrides = WallpaperSettings {
        scale_mode: Some(ScaleMode::Contain),
        aspect_ratio: Some(AspectRatio::Ratio21x9),
        anti_aliasing: Some(AntiAliasing::High),
        frame_rate: Some(24),
        hardware_decoding: Some(false),
        muted: Some(false),
        volume: Some(35),
    };

    core.update_wallpaper_settings(&media_id, overrides)
        .expect("set wallpaper settings");
    let effective = core
        .effective_settings(&media_id)
        .expect("effective settings");
    assert_eq!(effective.scale_mode, ScaleMode::Contain);
    assert_eq!(effective.aspect_ratio, AspectRatio::Ratio21x9);
    assert_eq!(effective.anti_aliasing, AntiAliasing::High);
    assert_eq!(effective.frame_rate, 24);
    assert!(!effective.hardware_decoding);
    assert!(!effective.default_muted);
    assert_eq!(effective.volume, 35);

    core.play(&media_id).expect("play with overrides");
    assert!(!core.snapshot().playback.muted);
    assert_eq!(core.snapshot().playback.volume, 35);

    core.update_wallpaper_settings(&media_id, WallpaperSettings::default())
        .expect("restore global settings");
    assert_eq!(
        core.effective_settings(&media_id)
            .expect("global")
            .frame_rate,
        60
    );
    fs::remove_dir_all(root).expect("clean test directory");
}

#[test]
fn display_layout_validates_selection_and_records_the_playback_target() {
    let root = test_root("display-layout");
    let video = root.join("ocean.mp4");
    fs::write(&video, b"video").expect("create video");
    let mut snapshot = AppSnapshot {
        displays: vec![display("left", -1920, true), display("right", 0, false)],
        ..Default::default()
    };
    let media_id = import_media(&mut snapshot, &[video]).expect("import")[0]
        .id
        .clone();
    let mut core = WallCore::new(snapshot);

    assert_eq!(
        core.set_display_layout(
            DisplayMode::Independent,
            vec!["left".to_owned(), "right".to_owned()]
        )
        .expect_err("independent accepts one display")
        .code,
        "invalid_display_selection"
    );
    core.set_display_layout(
        DisplayMode::Clone,
        vec!["left".to_owned(), "right".to_owned()],
    )
    .expect("select clone group");
    core.play(&media_id).expect("play clone group");

    let assignment = &core.snapshot().playback.display_assignments[0];
    assert_eq!(assignment.mode, DisplayMode::Clone);
    assert_eq!(assignment.display_ids, vec!["left", "right"]);
    assert_eq!(assignment.wallpaper_id, media_id);

    let target_id = assignment.target_id.clone();
    core.toggle_target_pause(&target_id).expect("pause target");
    assert_eq!(
        core.snapshot().playback.display_assignments[0].status,
        PlaybackStatus::Paused
    );
    core.set_target_pause_reason(&target_id, PauseReason::Fullscreen, true)
        .expect("fullscreen pause");
    core.set_target_pause_reason(&target_id, PauseReason::Fullscreen, false)
        .expect("remove fullscreen pause");
    assert_eq!(
        core.snapshot().playback.display_assignments[0].status,
        PlaybackStatus::Paused
    );
    core.set_target_muted(&target_id, false)
        .expect("unmute target");
    assert!(!core.snapshot().playback.display_assignments[0].muted);
    core.stop_target(&target_id).expect("stop target");
    assert!(core.snapshot().playback.display_assignments.is_empty());
    fs::remove_dir_all(root).expect("clean test directory");
}

#[test]
fn a_display_can_only_belong_to_one_active_target() {
    let root = test_root("display-exclusive");
    let first_video = root.join("first.mp4");
    let second_video = root.join("second.mp4");
    fs::write(&first_video, b"first").expect("create first video");
    fs::write(&second_video, b"second").expect("create second video");
    let mut snapshot = AppSnapshot {
        displays: vec![display("left", -1920, true), display("right", 0, false)],
        ..Default::default()
    };
    let imported =
        import_media(&mut snapshot, &[first_video, second_video]).expect("import videos");
    let first_id = imported[0].id.clone();
    let second_id = imported[1].id.clone();
    let mut core = WallCore::new(snapshot);

    core.set_display_layout(DisplayMode::Independent, vec!["left".to_owned()])
        .expect("select left display");
    core.play(&first_id).expect("play first display target");
    core.set_display_layout(
        DisplayMode::Clone,
        vec!["left".to_owned(), "right".to_owned()],
    )
    .expect("select clone target");
    core.play(&second_id).expect("replace overlapping target");

    assert_eq!(core.snapshot().playback.display_assignments.len(), 1);
    let assignment = &core.snapshot().playback.display_assignments[0];
    assert_eq!(assignment.mode, DisplayMode::Clone);
    assert_eq!(assignment.wallpaper_id, second_id);
    fs::remove_dir_all(root).expect("clean test directory");
}

#[test]
fn removing_one_running_wallpaper_preserves_other_display_targets() {
    let root = test_root("remove-running-target");
    let first_video = root.join("first.mp4");
    let second_video = root.join("second.mp4");
    fs::write(&first_video, b"first").expect("create first video");
    fs::write(&second_video, b"second").expect("create second video");
    let mut snapshot = AppSnapshot {
        displays: vec![display("left", -1920, true), display("right", 0, false)],
        ..Default::default()
    };
    let imported =
        import_media(&mut snapshot, &[first_video, second_video]).expect("import videos");
    let first_id = imported[0].id.clone();
    let second_id = imported[1].id.clone();
    let mut core = WallCore::new(snapshot);
    core.set_display_layout(DisplayMode::Independent, vec!["left".to_owned()])
        .expect("select left");
    core.play(&first_id).expect("play left");
    core.set_display_layout(DisplayMode::Independent, vec!["right".to_owned()])
        .expect("select right");
    core.play(&second_id).expect("play right");

    core.remove(&second_id).expect("remove right wallpaper");

    assert_eq!(core.snapshot().playback.display_assignments.len(), 1);
    assert_eq!(
        core.snapshot().playback.display_assignments[0].wallpaper_id,
        first_id
    );
    assert_eq!(
        core.snapshot().playback.active_id.as_deref(),
        Some(first_id.as_str())
    );
    assert_eq!(core.snapshot().playback.status, PlaybackStatus::Playing);
    fs::remove_dir_all(root).expect("clean test directory");
}

#[test]
fn media_settings_only_update_matching_targets_and_preserve_pause_reasons() {
    let root = test_root("media-target-settings");
    let first_video = root.join("first.mp4");
    let second_video = root.join("second.mp4");
    fs::write(&first_video, b"first").expect("create first video");
    fs::write(&second_video, b"second").expect("create second video");
    let mut snapshot = AppSnapshot {
        displays: vec![display("left", -1920, true), display("right", 0, false)],
        ..Default::default()
    };
    let imported =
        import_media(&mut snapshot, &[first_video, second_video]).expect("import videos");
    let first_id = imported[0].id.clone();
    let second_id = imported[1].id.clone();
    let mut core = WallCore::new(snapshot);
    core.set_display_layout(DisplayMode::Independent, vec!["left".to_owned()])
        .expect("select left");
    core.play(&first_id).expect("play left");
    let left_target = core.snapshot().playback.display_assignments[0]
        .target_id
        .clone();
    core.toggle_target_pause(&left_target).expect("pause left");
    core.set_display_layout(DisplayMode::Independent, vec!["right".to_owned()])
        .expect("select right");
    core.play(&second_id).expect("play right");

    core.set_media_playback_settings(&first_id, false, 35)
        .expect("update first media target");

    let left = core
        .snapshot()
        .playback
        .display_assignments
        .iter()
        .find(|assignment| assignment.wallpaper_id == first_id)
        .expect("left assignment");
    let right = core
        .snapshot()
        .playback
        .display_assignments
        .iter()
        .find(|assignment| assignment.wallpaper_id == second_id)
        .expect("right assignment");
    assert!(!left.muted);
    assert_eq!(left.volume, 35);
    assert_eq!(left.status, PlaybackStatus::Paused);
    assert!(left.pause_reasons.contains(&PauseReason::Manual));
    assert!(right.muted);
    assert_eq!(right.volume, 0);
    assert_eq!(right.status, PlaybackStatus::Playing);
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

fn display(id: &str, x: i32, primary: bool) -> DisplayInfo {
    DisplayInfo {
        id: id.to_owned(),
        name: id.to_owned(),
        x,
        y: 0,
        width: 1920,
        height: 1080,
        primary,
        connected: true,
    }
}
