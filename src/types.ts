/** 定义 Vue 与 Rust 命令之间共享的序列化类型。 */

export type MediaKind = 'video' | 'image';
export type ScaleMode = 'cover' | 'contain' | 'stretch';
export type PlaybackStatus = 'idle' | 'playing' | 'paused' | 'error';
export type PauseReason = 'manual' | 'fullscreen' | 'battery' | 'displaySleep';

export interface WallpaperItem {
    id: string;
    name: string;
    path: string;
    kind: MediaKind;
    format: string;
    width: number | null;
    height: number | null;
    durationSeconds: number | null;
    thumbnailPath: string | null;
    missing: boolean;
}

export interface AppSettings {
    autoStart: boolean;
    closeToTray: boolean;
    restoreLastWallpaper: boolean;
    language: 'zh-CN';
    scaleMode: ScaleMode;
    frameRate: 30 | 60;
    hardwareDecoding: boolean;
    defaultMuted: boolean;
    volume: number;
    pauseOnFullscreen: boolean;
    pauseOnBattery: boolean;
    pauseOnDisplaySleep: boolean;
}

export interface PlaybackState {
    activeId: string | null;
    status: PlaybackStatus;
    muted: boolean;
    volume: number;
    pauseReasons: PauseReason[];
    lastError: string | null;
}

export interface AppSnapshot {
    library: WallpaperItem[];
    settings: AppSettings;
    playback: PlaybackState;
}

export interface AppError {
    code: string;
    message: string;
    recoverable: boolean;
}
