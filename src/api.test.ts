/** 验证前端调用 Tauri 时使用 Rust 命令绑定要求的 camelCase 参数名。 */
// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AppSnapshot } from './types';

const { invokeMock } = vi.hoisted(() => ({
    invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
    convertFileSrc: vi.fn(),
    invoke: invokeMock,
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

import {
    createCategory,
    deleteCategory,
    openMediaFolder,
    play,
    relocateMedia,
    removeMedia,
    removeMediaBatch,
    removeMissingMedia,
    renameCategory,
    scanLibrary,
    setCategoryMembership,
    setDisplayLayout,
    setWallpaperSettings,
    stopMedia,
    toggleMediaPause,
} from './api';

describe('Tauri command contract', () => {
    beforeEach(() => {
        Object.assign(window, { __TAURI_INTERNALS__: {} });
        invokeMock.mockReset();
        invokeMock.mockResolvedValue(snapshot());
    });

    it('passes media identifiers using camelCase keys', async () => {
        await play('video-1');
        await removeMedia('video-1');
        await relocateMedia('video-1', 'D:\\Wallpapers\\video.mp4');
        await openMediaFolder('video-1');

        expect(invokeMock).toHaveBeenNthCalledWith(1, 'play', { mediaId: 'video-1' });
        expect(invokeMock).toHaveBeenNthCalledWith(2, 'remove_media', { mediaId: 'video-1' });
        expect(invokeMock).toHaveBeenNthCalledWith(3, 'relocate_media', {
            mediaId: 'video-1',
            path: 'D:\\Wallpapers\\video.mp4',
        });
        expect(invokeMock).toHaveBeenNthCalledWith(4, 'open_media_folder', { mediaId: 'video-1' });
    });

    it('uses one snapshot command contract for category mutations', async () => {
        await createCategory('自然风景');
        await renameCategory('nature', '森林');
        await setCategoryMembership(['video-1', 'video-2'], 'nature', true);
        await deleteCategory('nature');

        expect(invokeMock).toHaveBeenNthCalledWith(1, 'create_category', { name: '自然风景' });
        expect(invokeMock).toHaveBeenNthCalledWith(2, 'rename_category', {
            categoryId: 'nature',
            name: '森林',
        });
        expect(invokeMock).toHaveBeenNthCalledWith(3, 'set_category_membership', {
            mediaIds: ['video-1', 'video-2'],
            categoryId: 'nature',
            assigned: true,
        });
        expect(invokeMock).toHaveBeenNthCalledWith(4, 'delete_category', { categoryId: 'nature' });
    });

    it('uses snapshot commands for scanning and library-only removal', async () => {
        await scanLibrary();
        await removeMediaBatch(['video-1', 'video-2']);
        await removeMissingMedia();

        expect(invokeMock).toHaveBeenNthCalledWith(1, 'scan_library', undefined);
        expect(invokeMock).toHaveBeenNthCalledWith(2, 'remove_media_batch', {
            mediaIds: ['video-1', 'video-2'],
        });
        expect(invokeMock).toHaveBeenNthCalledWith(3, 'remove_missing_media', undefined);
    });

    it('updates one wallpaper override with the shared settings contract', async () => {
        await setWallpaperSettings('video-1', { frameRate: 24, muted: false });

        expect(invokeMock).toHaveBeenCalledWith('set_wallpaper_settings', {
            mediaId: 'video-1',
            settings: { frameRate: 24, muted: false },
        });
    });

    it('sets the selected display layout with camelCase identifiers', async () => {
        await setDisplayLayout('span', ['left', 'right']);

        expect(invokeMock).toHaveBeenCalledWith('set_display_layout', {
            mode: 'span',
            displayIds: ['left', 'right'],
        });
    });

    it('controls every target using one wallpaper-scoped command', async () => {
        await toggleMediaPause('video-1');
        await stopMedia('video-1');

        expect(invokeMock).toHaveBeenNthCalledWith(1, 'toggle_media_pause', { mediaId: 'video-1' });
        expect(invokeMock).toHaveBeenNthCalledWith(2, 'stop_media', { mediaId: 'video-1' });
    });
});

function snapshot(): AppSnapshot {
    return {
        library: [],
        categories: [],
        settings: {
            autoStart: false,
            closeToTray: true,
            restoreLastWallpaper: true,
            language: 'zh-CN',
            scaleMode: 'cover',
            frameRate: 60,
            aspectRatio: 'original',
            antiAliasing: 'balanced',
            hardwareDecoding: true,
            defaultMuted: true,
            volume: 0,
            pauseOnFullscreen: true,
            pauseOnMaximized: true,
            pauseOnBattery: true,
            pauseOnDisplaySleep: true,
            displayMode: 'independent',
            selectedDisplayIds: [],
        },
        playback: {
            activeId: null,
            status: 'idle',
            muted: true,
            volume: 0,
            pauseReasons: [],
            lastError: null,
        },
    };
}
