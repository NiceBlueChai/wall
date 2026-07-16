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

import { openMediaFolder, play, relocateMedia, removeMedia } from './api';

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
});

function snapshot(): AppSnapshot {
    return {
        library: [],
        settings: {
            autoStart: false,
            closeToTray: true,
            restoreLastWallpaper: true,
            language: 'zh-CN',
            scaleMode: 'cover',
            frameRate: 60,
            hardwareDecoding: true,
            defaultMuted: true,
            volume: 0,
            pauseOnFullscreen: true,
            pauseOnBattery: true,
            pauseOnDisplaySleep: true,
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
