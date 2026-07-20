//! 验证 mpv 启动参数保持离线、无交互并匹配 Wall 设置。

use std::path::Path;
use wall_lib::model::{AntiAliasing, AppSettings, AspectRatio, MediaKind, ScaleMode};
use wall_lib::player::{
    ScreenRegion, build_live_settings_commands, build_mpv_arguments, drift_correction_target,
    union_regions,
};

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
    assert!(arguments.contains(&"--video-aspect-override=-1".to_owned()));
    assert!(arguments.contains(&"--scale=spline36".to_owned()));
    assert!(arguments.contains(&"--vf=fps=60".to_owned()));
}

#[test]
fn source_fps_aspect_and_quality_map_without_extra_filters() {
    let settings = AppSettings {
        frame_rate: 0,
        aspect_ratio: AspectRatio::Ratio21x9,
        anti_aliasing: AntiAliasing::High,
        ..Default::default()
    };

    let arguments = build_mpv_arguments(
        Path::new("cinema.mp4"),
        MediaKind::Video,
        "wall-cinema-test",
        3440,
        1440,
        "wall",
        &settings,
    );

    assert!(
        !arguments
            .iter()
            .any(|argument| argument.starts_with("--vf=fps="))
    );
    assert!(arguments.contains(&"--video-aspect-override=21:9".to_owned()));
    assert!(arguments.contains(&"--scale=ewa_lanczossharp".to_owned()));
}

#[test]
fn live_visual_commands_clear_the_fps_filter_for_source_rate() {
    let settings = AppSettings {
        frame_rate: 0,
        aspect_ratio: AspectRatio::Screen,
        anti_aliasing: AntiAliasing::Off,
        ..Default::default()
    };

    let commands = build_live_settings_commands(&settings, 2560, 1440, MediaKind::Video);

    assert!(commands.contains(&serde_json::json!({
        "command": ["set_property", "video-aspect-override", "2560:1440"]
    })));
    assert!(commands.contains(&serde_json::json!({
        "command": ["set_property", "scale", "bilinear"]
    })));
    assert!(commands.contains(&serde_json::json!({
        "command": ["set_property", "vf", ""]
    })));
}

#[test]
fn image_live_settings_clear_the_video_fps_filter() {
    let commands =
        build_live_settings_commands(&AppSettings::default(), 2560, 1440, MediaKind::Image);

    assert!(commands.contains(&serde_json::json!({
        "command": ["set_property", "vf", ""]
    })));
}

#[test]
fn span_geometry_covers_negative_and_positive_monitor_coordinates() {
    let region = union_regions(&[
        ScreenRegion {
            x: -1920,
            y: 0,
            width: 1920,
            height: 1080,
        },
        ScreenRegion {
            x: 0,
            y: -200,
            width: 2560,
            height: 1440,
        },
    ])
    .expect("union region");

    assert_eq!(
        region,
        ScreenRegion {
            x: -1920,
            y: -200,
            width: 4480,
            height: 1440
        }
    );
}

#[test]
fn clone_sync_only_corrects_drift_above_one_hundred_milliseconds() {
    assert_eq!(drift_correction_target(&[10.0, 10.05]), None);
    assert_eq!(drift_correction_target(&[10.0, 10.2]), Some(10.2));
}

#[test]
fn image_and_stretch_arguments_are_explicit() {
    let settings = AppSettings {
        scale_mode: ScaleMode::Stretch,
        hardware_decoding: false,
        ..Default::default()
    };

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
    assert!(
        !arguments
            .iter()
            .any(|argument| argument.starts_with("--vf=fps="))
    );
}
