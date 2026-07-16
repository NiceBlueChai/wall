//! 验证 mpv 启动参数保持离线、无交互并匹配 Wall 设置。

use std::path::Path;
use wall_lib::model::{AppSettings, MediaKind, ScaleMode};
use wall_lib::player::build_mpv_arguments;

#[test]
fn video_arguments_enable_loop_ipc_and_desktop_embedding() {
    let settings = AppSettings::default();

    let arguments = build_mpv_arguments(
        Path::new(r"D:\Wallpapers\ocean.mp4"),
        MediaKind::Video,
        "wall-wallpaper-test",
        1920,
        1080,
        r"\\.\pipe\wall-mpv",
        &settings,
    );

    assert!(arguments.contains(&"--no-config".to_owned()));
    assert!(arguments.contains(&"--loop-file=inf".to_owned()));
    assert!(arguments.contains(&"--force-window=immediate".to_owned()));
    assert!(arguments.contains(&"--no-border".to_owned()));
    assert!(arguments.contains(&"--title=wall-wallpaper-test".to_owned()));
    assert!(arguments.contains(&"--geometry=1920x1080+0+0".to_owned()));
    assert!(
        !arguments
            .iter()
            .any(|argument| argument.starts_with("--wid="))
    );
    assert!(arguments.contains(&r"--input-ipc-server=\\.\pipe\wall-mpv".to_owned()));
    assert!(arguments.contains(&"--hwdec=auto-safe".to_owned()));
    assert!(arguments.contains(&"--panscan=1.0".to_owned()));
}

#[test]
fn image_and_stretch_arguments_are_explicit() {
    let mut settings = AppSettings::default();
    settings.scale_mode = ScaleMode::Stretch;
    settings.hardware_decoding = false;

    let arguments = build_mpv_arguments(
        Path::new("aurora.png"),
        MediaKind::Image,
        "wall-image-test",
        1280,
        720,
        "wall",
        &settings,
    );

    assert!(arguments.contains(&"--image-display-duration=inf".to_owned()));
    assert!(arguments.contains(&"--keepaspect=no".to_owned()));
    assert!(arguments.contains(&"--hwdec=no".to_owned()));
}
