//! 负责 Wall 用户数据目录中的 JSON 持久化。

use crate::model::AppSnapshot;
use serde::{Serialize, de::DeserializeOwned};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("无法访问本地数据文件：{0}")]
    Io(#[from] std::io::Error),
    #[error("无法序列化本地数据：{0}")]
    Json(#[from] serde_json::Error),
}

pub struct Storage {
    root: PathBuf,
}

impl Storage {
    /// 创建指向应用数据目录的存储实例。
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// 分别加载媒体库、设置和上次播放会话；损坏的单个文件回退为默认值。
    pub fn load(&self) -> Result<AppSnapshot, StorageError> {
        Ok(AppSnapshot {
            library: load_or_default(&self.root.join("library.json"))?,
            settings: load_or_default(&self.root.join("settings.json"))?,
            playback: load_or_default(&self.root.join("session.json"))?,
        })
    }

    /// 将快照分别持久化到三个可独立恢复的 JSON 文件。
    pub fn save(&self, snapshot: &AppSnapshot) -> Result<(), StorageError> {
        fs::create_dir_all(&self.root)?;
        save_atomic(&self.root.join("library.json"), &snapshot.library)?;
        save_atomic(&self.root.join("settings.json"), &snapshot.settings)?;
        save_atomic(&self.root.join("session.json"), &snapshot.playback)?;
        Ok(())
    }
}

fn load_or_default<T>(path: &Path) -> Result<T, StorageError>
where
    T: Default + DeserializeOwned,
{
    match fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes).unwrap_or_default()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(error) => Err(error.into()),
    }
}

fn save_atomic<T>(path: &Path, value: &T) -> Result<(), StorageError>
where
    T: Serialize + ?Sized,
{
    let temporary = path.with_extension("json.tmp");
    let mut file = File::create(&temporary)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.sync_all()?;
    replace_file(&temporary, path)?;
    Ok(())
}

#[cfg(windows)]
fn replace_file(source: &Path, target: &Path) -> Result<(), StorageError> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    use windows::core::PCWSTR;

    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(target.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(source: &Path, target: &Path) -> Result<(), StorageError> {
    fs::rename(source, target)?;
    Ok(())
}
