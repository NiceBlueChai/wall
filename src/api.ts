/** 封装 Wall 的 Tauri 命令与状态事件，保持视图只消费 AppSnapshot。 */
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AppSettings, AppSnapshot, DisplayMode, ScaleMode, WallpaperSettings } from './types';
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
export const toggleTargetPause = (targetId: string) => snapshotCommand('toggle_target_pause', { targetId });
/** 原子地切换使用指定壁纸的全部目标的手动暂停状态。 */
export const toggleMediaPause = (mediaId: string) => snapshotCommand('toggle_media_pause', { mediaId });
export const stop = () => snapshotCommand('stop');
export const stopTarget = (targetId: string) => snapshotCommand('stop_target', { targetId });
/** 原子地停止使用指定壁纸的全部显示目标。 */
export const stopMedia = (mediaId: string) => snapshotCommand('stop_media', { mediaId });
export const setMuted = (muted: boolean) => snapshotCommand('set_muted', { muted });
export const setVolume = (volume: number) => snapshotCommand('set_volume', { volume });
export const setScaleMode = (mode: ScaleMode) => snapshotCommand('set_scale_mode', { mode });
export const removeMedia = (mediaId: string) => snapshotCommand('remove_media', { mediaId });
/** 原子地移除多条壁纸库记录，不修改源文件。 */
export const removeMediaBatch = (mediaIds: string[]) => snapshotCommand('remove_media_batch', { mediaIds });
/** 重新扫描整个壁纸库并刷新文件失效状态。 */
export const scanLibrary = () => snapshotCommand('scan_library');
/** 重新扫描后移除全部失效记录，不修改源文件。 */
export const removeMissingMedia = () => snapshotCommand('remove_missing_media');
export const relocateMedia = (mediaId: string, path: string) => snapshotCommand('relocate_media', { mediaId, path });
export const createCategory = (name: string) => snapshotCommand('create_category', { name });
export const renameCategory = (categoryId: string, name: string) =>
    snapshotCommand('rename_category', { categoryId, name });
export const deleteCategory = (categoryId: string) => snapshotCommand('delete_category', { categoryId });
export const setCategoryMembership = (mediaIds: string[], categoryId: string, assigned: boolean) =>
    snapshotCommand('set_category_membership', { mediaIds, categoryId, assigned });
export const setWallpaperSettings = (mediaId: string, settings: WallpaperSettings) =>
    snapshotCommand('set_wallpaper_settings', { mediaId, settings });
export const setDisplayLayout = (mode: DisplayMode, displayIds: string[]) =>
    snapshotCommand('set_display_layout', { mode, displayIds });
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
