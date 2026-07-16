//! 验证并登记用户选择的本地视频和图片。

use crate::model::{AppError, AppSnapshot, MediaKind, WallpaperItem};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "webm", "mov", "avi"];
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp", "gif"];

/// 验证媒体路径并将尚未登记的项目加入快照。
pub fn import_media(
    snapshot: &mut AppSnapshot,
    paths: &[PathBuf],
) -> Result<Vec<WallpaperItem>, AppError> {
    let mut candidates = Vec::with_capacity(paths.len());
    for path in paths {
        let canonical = path
            .canonicalize()
            .map_err(|_| error("file_missing", "所选文件不存在或无法访问"))?;
        let kind = classify_media(&canonical).ok_or_else(|| {
            error(
                "unsupported_format",
                "不支持该格式，请选择 MP4、MKV、WebM、MOV、AVI、JPG、PNG、WebP、BMP 或 GIF",
            )
        })?;
        candidates.push((canonical, kind));
    }

    let mut known: HashSet<String> = snapshot
        .library
        .iter()
        .map(|item| path_key(Path::new(&item.path)))
        .collect();
    let mut imported = Vec::new();
    for (path, kind) in candidates {
        if !known.insert(path_key(&path)) {
            continue;
        }
        let item = wallpaper_item(path, kind);
        snapshot.library.push(item.clone());
        imported.push(item);
    }
    Ok(imported)
}

/// 根据允许列表判断本地文件的媒体类别。
pub fn classify_media(path: &Path) -> Option<MediaKind> {
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Video)
    } else if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Image)
    } else {
        None
    }
}

fn wallpaper_item(path: PathBuf, kind: MediaKind) -> WallpaperItem {
    let name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("未命名壁纸")
        .to_owned();
    let format = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_uppercase();
    WallpaperItem {
        id: Uuid::new_v4().to_string(),
        name,
        path: path.to_string_lossy().into_owned(),
        kind,
        format,
        width: None,
        height: None,
        duration_seconds: None,
        thumbnail_path: None,
        missing: false,
    }
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

fn error(code: &str, message: &str) -> AppError {
    AppError {
        code: code.to_owned(),
        message: message.to_owned(),
        recoverable: true,
    }
}
