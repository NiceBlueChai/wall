/** 封装 Wall 的 Tauri 命令与状态事件，保持视图只消费 AppSnapshot。 */
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AppSettings, AppSnapshot, ScaleMode } from './types';
import { wallStore } from './store';

/** 从 Rust 恢复完整应用快照。 */
export async function bootstrap(): Promise<AppSnapshot> {
    if (!isTauri()) {
        return wallStore.snapshot;
    }
    const snapshot = await invoke<AppSnapshot>('bootstrap');
    wallStore.applySnapshot(snapshot);
    return snapshot;
}

/** 订阅 Rust、托盘和系统监视器产生的状态变化。 */
export async function listenToSnapshots(): Promise<() => void> {
    if (!isTauri()) {
        return () => undefined;
    }
    return listen<AppSnapshot>('app-state://changed', (event) => wallStore.applySnapshot(event.payload));
}

export async function importMedia(paths: string[]) {
    await invoke('import_media', { paths });
    return bootstrap();
}

export const play = (mediaId: string) => snapshotCommand('play', { mediaId });
export const togglePause = () => snapshotCommand('toggle_pause');
export const stop = () => snapshotCommand('stop');
export const setMuted = (muted: boolean) => snapshotCommand('set_muted', { muted });
export const setVolume = (volume: number) => snapshotCommand('set_volume', { volume });
export const setScaleMode = (mode: ScaleMode) => snapshotCommand('set_scale_mode', { mode });
export const removeMedia = (mediaId: string) => snapshotCommand('remove_media', { mediaId });
export const relocateMedia = (mediaId: string, path: string) => snapshotCommand('relocate_media', { mediaId, path });
export const updateSettings = (settings: AppSettings) => snapshotCommand('update_settings', { settings });
export const openMediaFolder = (mediaId: string) => invoke('open_media_folder', { mediaId });
export const openLogs = () => invoke('open_logs');
export const openLicense = () => invoke('open_license');
export const openProjectHomepage = () => invoke('open_project_homepage');

/** 将已导入的本地路径转换为 WebView 可显示的资源 URL。 */
export function mediaUrl(path: string): string {
    return isTauri() ? convertFileSrc(path) : '';
}

async function snapshotCommand(command: string, args?: Record<string, unknown>): Promise<AppSnapshot> {
    const snapshot = await invoke<AppSnapshot>(command, args);
    wallStore.applySnapshot(snapshot);
    return snapshot;
}

function isTauri(): boolean {
    return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
