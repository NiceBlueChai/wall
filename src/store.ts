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
    };
}

/** 创建可由 Tauri 快照整体更新的轻量状态容器。 */
export function createWallStore() {
    const state = reactive({
        snapshot: emptySnapshot(),
        search: '',
        filter: 'all' as LibraryFilter,
        activeCategoryId: null as string | null,
        batchMode: false,
        selectedMediaIds: [] as string[],
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
        get activeCategoryId() {
            return state.activeCategoryId;
        },
        set activeCategoryId(value: string | null) {
            state.activeCategoryId = value;
        },
        get batchMode() {
            return state.batchMode;
        },
        get selectedMediaIds() {
            return state.selectedMediaIds;
        },
        get filteredLibrary(): WallpaperItem[] {
            const query = state.search.trim().toLocaleLowerCase();
            return state.snapshot.library.filter((item) => {
                const matchesKind = state.filter === 'all' || item.kind === state.filter;
                const matchesSearch = !query || item.name.toLocaleLowerCase().includes(query);
                const matchesCategory =
                    state.activeCategoryId === null || item.categoryIds.includes(state.activeCategoryId);
                return matchesKind && matchesSearch && matchesCategory;
            });
        },
        applySnapshot(snapshot: AppSnapshot) {
            state.snapshot = snapshot;
            const mediaIds = new Set(snapshot.library.map((item) => item.id));
            state.selectedMediaIds = state.selectedMediaIds.filter((id) => mediaIds.has(id));
            if (state.activeCategoryId && !snapshot.categories.some((item) => item.id === state.activeCategoryId)) {
                state.activeCategoryId = null;
            }
        },
        enterBatchMode() {
            state.batchMode = true;
            state.selectedMediaIds = [];
        },
        exitBatchMode() {
            state.batchMode = false;
            state.selectedMediaIds = [];
        },
        toggleMediaSelection(mediaId: string) {
            state.selectedMediaIds = state.selectedMediaIds.includes(mediaId)
                ? state.selectedMediaIds.filter((id) => id !== mediaId)
                : [...state.selectedMediaIds, mediaId];
        },
    };
}

function emptySnapshot(): AppSnapshot {
    return {
        library: [],
        categories: [],
        settings: defaultSettings(),
        playback: {
            activeId: null,
            status: 'idle',
            muted: true,
            volume: 0,
            pauseReasons: [],
            lastError: null,
        },
        displays: [],
    };
}

export type WallStore = ReturnType<typeof createWallStore>;
export const wallStore = createWallStore();
