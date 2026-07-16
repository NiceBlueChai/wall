//! 验证本地媒体导入边界和重复路径处理。

use std::fs;
use wall_lib::media::import_media;
use wall_lib::model::{AppSnapshot, MediaKind};

#[test]
fn imports_supported_media_without_copying_source() {
    let root = test_root("supported");
    let video = root.join("Ocean Loop.mp4");
    let image = root.join("Aurora.PNG");
    fs::write(&video, b"video").expect("create video");
    fs::write(&image, b"image").expect("create image");
    let mut snapshot = AppSnapshot::default();

    let imported =
        import_media(&mut snapshot, &[video.clone(), image.clone()]).expect("import media");

    assert_eq!(imported.len(), 2);
    assert_eq!(imported[0].kind, MediaKind::Video);
    assert_eq!(imported[1].kind, MediaKind::Image);
    assert_eq!(fs::read(video).expect("source remains"), b"video");
    fs::remove_dir_all(root).expect("clean test directory");
}

#[test]
fn ignores_paths_already_in_library() {
    let root = test_root("duplicate");
    let video = root.join("ocean.mp4");
    fs::write(&video, b"video").expect("create video");
    let mut snapshot = AppSnapshot::default();
    import_media(&mut snapshot, std::slice::from_ref(&video)).expect("first import");

    let imported = import_media(&mut snapshot, &[video]).expect("second import");

    assert!(imported.is_empty());
    assert_eq!(snapshot.library.len(), 1);
    fs::remove_dir_all(root).expect("clean test directory");
}

#[test]
fn rejects_unsupported_files() {
    let root = test_root("unsupported");
    let text = root.join("notes.txt");
    fs::write(&text, b"notes").expect("create text file");

    let error =
        import_media(&mut AppSnapshot::default(), &[text]).expect_err("reject unsupported file");

    assert_eq!(error.code, "unsupported_format");
    assert!(error.recoverable);
    fs::remove_dir_all(root).expect("clean test directory");
}

fn test_root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("wall-media-{name}-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove stale test directory");
    }
    fs::create_dir_all(&root).expect("create test directory");
    root
}
