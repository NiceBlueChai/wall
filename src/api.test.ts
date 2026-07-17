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
    renameCategory,
    setCategoryMembership,
    setDisplayLayout,
    setWallpaperSettings,
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
