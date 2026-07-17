/** 定义 Vue 与 Rust 命令之间共享的序列化类型。 */

export type MediaKind = 'video' | 'image';
export type ScaleMode = 'cover' | 'contain' | 'stretch';
export type AspectRatio =
    | 'original'
    | 'screen'
    | 'ratio16x9'
    | 'ratio16x10'
    | 'ratio21x9'
    | 'ratio32x9'
    | 'ratio4x3'
    | 'ratio1x1'
    | 'ratio9x16';
export type AntiAliasing = 'off' | 'balanced' | 'high';
export type DisplayMode = 'independent' | 'clone' | 'span';
export type PlaybackStatus = 'idle' | 'playing' | 'paused' | 'error';
export type PauseReason = 'manual' | 'fullscreen' | 'maximized' | 'battery' | 'displaySleep';

export interface Category {
    id: string;
    name: string;
}

export interface WallpaperSettings {
    scaleMode?: ScaleMode;
    aspectRatio?: AspectRatio;
    antiAliasing?: AntiAliasing;
    frameRate?: 0 | 24 | 30 | 60;
    hardwareDecoding?: boolean;
    muted?: boolean;
    volume?: number;
}

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
    categoryIds: string[];
    settings?: WallpaperSettings;
}

export interface AppSettings {
    autoStart: boolean;
    closeToTray: boolean;
    restoreLastWallpaper: boolean;
    language: 'zh-CN';
    scaleMode: ScaleMode;
    frameRate: 0 | 24 | 30 | 60;
    aspectRatio: AspectRatio;
    antiAliasing: AntiAliasing;
    hardwareDecoding: boolean;
    defaultMuted: boolean;
    volume: number;
    pauseOnFullscreen: boolean;
    pauseOnMaximized: boolean;
    pauseOnBattery: boolean;
    pauseOnDisplaySleep: boolean;
    displayMode: DisplayMode;
    selectedDisplayIds: string[];
}

export interface DisplayInfo {
    id: string;
    name: string;
    x: number;
    y: number;
    width: number;
    height: number;
    primary: boolean;
    connected: boolean;
}

export interface DisplayAssignment {
    targetId: string;
    mode: DisplayMode;
    displayIds: string[];
    wallpaperId: string;
    status: PlaybackStatus;
    muted: boolean;
    volume: number;
    pauseReasons: PauseReason[];
}

export interface PlaybackState {
    activeId: string | null;
    status: PlaybackStatus;
    muted: boolean;
    volume: number;
    pauseReasons: PauseReason[];
    lastError: string | null;
    displayAssignments?: DisplayAssignment[];
}

export interface AppSnapshot {
    library: WallpaperItem[];
    categories: Category[];
    settings: AppSettings;
    playback: PlaybackState;
    displays?: DisplayInfo[];
}

export interface AppError {
    code: string;
    message: string;
    recoverable: boolean;
}
