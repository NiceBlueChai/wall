/** 验证前端状态的默认值、筛选和原生快照同步。 */
import { describe, expect, it } from 'vitest';
import { createWallStore, defaultSettings } from './store';
import type { AppSnapshot } from './types';

describe('Wall store', () => {
    it('uses safe offline defaults', () => {
        expect(defaultSettings()).toEqual({
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
        });
    });

    it('filters by search text and media kind', () => {
        const store = createWallStore();
        store.applySnapshot({
            library: [media('1', 'Ocean Loop', 'video'), media('2', 'Aurora', 'image')],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });

        store.search = 'ocean';
        expect(store.filteredLibrary.map((item) => item.id)).toEqual(['1']);
        store.search = '';
        store.filter = 'image';
        expect(store.filteredLibrary.map((item) => item.id)).toEqual(['2']);
    });
});

function media(id: string, name: string, kind: 'video' | 'image') {
    return {
        id,
        name,
        path: `D:\\Wallpapers\\${name}`,
        kind,
        format: kind === 'video' ? 'MP4' : 'PNG',
        width: 1920,
        height: 1080,
        durationSeconds: kind === 'video' ? 30 : null,
        thumbnailPath: null,
        missing: false,
    };
}

function idlePlayback(): AppSnapshot['playback'] {
    return {
        activeId: null,
        status: 'idle',
        muted: true,
        volume: 0,
        pauseReasons: [],
        lastError: null,
    };
}
