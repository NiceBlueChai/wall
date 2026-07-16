/** 保存 Wall 前端的单一响应式快照和本地筛选条件。 */
import { reactive } from 'vue';
import type { AppSettings, AppSnapshot, MediaKind, WallpaperItem } from './types';

type LibraryFilter = 'all' | MediaKind;

/** 返回首次运行时使用的安全离线设置。 */
export function defaultSettings(): AppSettings {
    return {
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
    };
}

/** 创建可由 Tauri 快照整体更新的轻量状态容器。 */
export function createWallStore() {
    const state = reactive({
        snapshot: emptySnapshot(),
        search: '',
        filter: 'all' as LibraryFilter,
    });

    return {
        get snapshot() {
            return state.snapshot;
        },
        get search() {
            return state.search;
        },
        set search(value: string) {
            state.search = value;
        },
        get filter() {
            return state.filter;
        },
        set filter(value: LibraryFilter) {
            state.filter = value;
        },
        get filteredLibrary(): WallpaperItem[] {
            const query = state.search.trim().toLocaleLowerCase();
            return state.snapshot.library.filter((item) => {
                const matchesKind = state.filter === 'all' || item.kind === state.filter;
                const matchesSearch = !query || item.name.toLocaleLowerCase().includes(query);
                return matchesKind && matchesSearch;
            });
        },
        applySnapshot(snapshot: AppSnapshot) {
            state.snapshot = snapshot;
        },
    };
}

function emptySnapshot(): AppSnapshot {
    return {
        library: [],
        settings: defaultSettings(),
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

export type WallStore = ReturnType<typeof createWallStore>;
export const wallStore = createWallStore();
