/** 验证前端状态的默认值、筛选和原生快照同步。 */
import { describe, expect, it, vi } from 'vitest';
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
        });
    });

    it('filters by search text and media kind', () => {
        const store = createWallStore();
        store.applySnapshot({
            library: [media('1', 'Ocean Loop', 'video'), media('2', 'Aurora', 'image')],
            categories: [],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });

        store.search = 'ocean';
        expect(store.filteredLibrary.map((item) => item.id)).toEqual(['1']);
        store.search = '';
        store.filter = 'image';
        expect(store.filteredLibrary.map((item) => item.id)).toEqual(['2']);
    });

    it('filters the library by the selected user category', () => {
        const store = createWallStore();
        const ocean = { ...media('1', 'Ocean Loop', 'video'), categoryIds: ['nature'] };
        const aurora = { ...media('2', 'Aurora', 'image'), categoryIds: ['space'] };
        store.applySnapshot({
            library: [ocean, aurora],
            categories: [
                { id: 'nature', name: '自然风景' },
                { id: 'space', name: '太空' },
            ],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });

        store.activeCategoryId = 'nature';

        expect(store.filteredLibrary.map((item) => item.id)).toEqual(['1']);
    });

    it('keeps batch selection in the store shared by the sidebar and library', () => {
        const store = createWallStore();
        store.enterBatchMode();
        store.toggleMediaSelection('1');
        store.toggleMediaSelection('2');
        store.toggleMediaSelection('1');

        expect(store.batchMode).toBe(true);
        expect(store.selectedMediaIds).toEqual(['2']);

        store.exitBatchMode();
        expect(store.batchMode).toBe(false);
        expect(store.selectedMediaIds).toEqual([]);
    });

    it('detects active media and exposes a short-lived notice', () => {
        vi.useFakeTimers();
        const store = createWallStore();
        store.applySnapshot({
            library: [media('1', 'Ocean Loop', 'video')],
            categories: [],
            settings: defaultSettings(),
            playback: {
                ...idlePlayback(),
                activeId: 'legacy',
                displayAssignments: [
                    {
                        targetId: 'display:primary',
                        mode: 'independent',
                        displayIds: ['primary'],
                        wallpaperId: '1',
                        status: 'playing',
                        muted: true,
                        volume: 0,
                        pauseReasons: [],
                    },
                ],
            },
        });

        expect(store.isMediaActive('1')).toBe(true);
        expect(store.isMediaActive('legacy')).toBe(false);
        store.showNotice('已从壁纸库移除');
        expect(store.notice).toBe('已从壁纸库移除');

        vi.advanceTimersByTime(3000);
        expect(store.notice).toBe('');
        vi.useRealTimers();
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
        categoryIds: [],
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
