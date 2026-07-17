//! 验证本地 JSON 存储的恢复、默认值和损坏文件隔离。

use std::fs;
use wall_lib::model::{AppSnapshot, MediaKind, WallpaperItem};
use wall_lib::storage::Storage;

#[test]
fn saves_and_loads_snapshot_across_three_files() {
    let root = test_root("roundtrip");
    let storage = Storage::new(root.clone());
    let mut snapshot = AppSnapshot::default();
    snapshot.library.push(sample_wallpaper());
    snapshot.settings.volume = 37;
    snapshot.playback.active_id = Some("ocean".to_owned());

    storage.save(&snapshot).expect("save snapshot");
    let loaded = storage.load().expect("load snapshot");

    assert_eq!(loaded, snapshot);
    assert!(root.join("library.json").is_file());
    assert!(root.join("settings.json").is_file());
    assert!(root.join("session.json").is_file());
    fs::remove_dir_all(root).expect("clean test directory");
}

#[test]
fn corrupted_settings_fall_back_without_losing_library() {
    let root = test_root("corrupt");
    let storage = Storage::new(root.clone());
    let mut snapshot = AppSnapshot::default();
    snapshot.library.push(sample_wallpaper());
    storage.save(&snapshot).expect("save snapshot");
    fs::write(root.join("settings.json"), b"not-json").expect("corrupt settings");

    let loaded = storage.load().expect("load recoverable snapshot");

    assert_eq!(loaded.library, snapshot.library);
    assert_eq!(loaded.settings, Default::default());
    fs::remove_dir_all(root).expect("clean test directory");
}

#[test]
fn library_file_contains_items_and_categories() {
    let root = test_root("library-state");
    let storage = Storage::new(root.clone());
    let mut snapshot = AppSnapshot::default();
    snapshot.library.push(sample_wallpaper());

    storage.save(&snapshot).expect("save snapshot");
    let library: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("library.json")).expect("read library file"))
            .expect("parse library file");

    assert!(library["items"].is_array());
    assert_eq!(library["categories"], serde_json::json!([]));
    fs::remove_dir_all(root).expect("clean test directory");
}

#[test]
fn loads_legacy_library_array_without_categories() {
    let root = test_root("legacy-library");
    fs::create_dir_all(&root).expect("create test directory");
    fs::write(
        root.join("library.json"),
        serde_json::to_vec_pretty(&vec![sample_wallpaper()]).expect("serialize legacy library"),
    )
    .expect("write legacy library");

    let loaded = Storage::new(root.clone())
        .load()
        .expect("load legacy library");

    assert_eq!(loaded.library, vec![sample_wallpaper()]);
    assert!(loaded.categories.is_empty());
    fs::remove_dir_all(root).expect("clean test directory");
}

fn sample_wallpaper() -> WallpaperItem {
    WallpaperItem {
        id: "ocean".to_owned(),
        name: "Ocean Loop".to_owned(),
        path: r"D:\Wallpapers\ocean.mp4".to_owned(),
        kind: MediaKind::Video,
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

fn test_root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("wall-storage-{name}-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove stale test directory");
    }
    root
}
