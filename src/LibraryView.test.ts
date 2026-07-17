/** 验证壁纸库的空状态和媒体卡片是同一响应式数据源。 */
// @vitest-environment jsdom

import { flushPromises, mount } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { describe, expect, it, vi } from 'vitest';
import { defaultSettings, wallStore } from './store';
import LibraryView from './views/LibraryView.vue';

const mocks = vi.hoisted(() => ({
    importMedia: vi.fn(),
    open: vi.fn(),
    play: vi.fn(),
    setCategoryMembership: vi.fn(),
    setDisplayLayout: vi.fn(),
}));

vi.mock('./api', () => ({
    importMedia: mocks.importMedia,
    mediaUrl: vi.fn(() => ''),
    play: mocks.play,
    setCategoryMembership: mocks.setCategoryMembership,
    setDisplayLayout: mocks.setDisplayLayout,
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: mocks.open }));

describe('LibraryView', () => {
    it('switches from the import empty state to a wallpaper card', async () => {
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/', component: LibraryView }],
        });
        await router.push('/');
        await router.isReady();
        const wrapper = mount(LibraryView, { global: { plugins: [router] } });
        expect(wrapper.text()).toContain('还没有壁纸');
        expect(wrapper.find('.empty-area').exists()).toBe(true);
        expect(wrapper.get('.empty-state img').attributes('src')).toBe('/icons/info.svg');
        expect(wrapper.find('.empty-state button').exists()).toBe(false);

        wallStore.applySnapshot({
            library: [
                {
                    id: 'ocean',
                    name: 'Ocean Loop',
                    path: 'D:\\Wallpapers\\ocean.mp4',
                    kind: 'video',
                    format: 'MP4',
                    width: 1920,
                    height: 1080,
                    durationSeconds: 30,
                    thumbnailPath: null,
                    missing: false,
                    categoryIds: [],
                },
            ],
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
        });
        await wrapper.vm.$nextTick();
        expect(wrapper.text()).toContain('Ocean Loop');
        expect(wrapper.text()).not.toContain('还没有壁纸');

        await wrapper.get('.wallpaper-card').trigger('dblclick');
        await flushPromises();
        expect(mocks.play).toHaveBeenCalledWith('ocean');

        mocks.open.mockResolvedValueOnce('D:\\Wallpapers\\new.mp4');
        await wrapper.get('.heading-actions .primary').trigger('click');
        await flushPromises();
        expect(mocks.importMedia).toHaveBeenCalledWith(['D:\\Wallpapers\\new.mp4']);

        wallStore.search = 'not-found';
        await wrapper.vm.$nextTick();
        expect(wrapper.text()).toContain('没有搜索结果');
        expect(wrapper.get('.compact-empty img').attributes('src')).toBe('/icons/search.svg');
    });

    it('assigns selected wallpapers through the confirmed batch category menu', async () => {
        wallStore.search = '';
        wallStore.filter = 'all';
        wallStore.activeCategoryId = null;
        wallStore.exitBatchMode();
        wallStore.applySnapshot({
            library: [
                {
                    id: 'ocean',
                    name: 'Ocean Loop',
                    path: 'D:\\Wallpapers\\ocean.mp4',
                    kind: 'video',
                    format: 'MP4',
                    width: 1920,
                    height: 1080,
                    durationSeconds: 30,
                    thumbnailPath: null,
                    missing: false,
                    categoryIds: ['nature'],
                },
            ],
            categories: [
                { id: 'nature', name: '自然风景' },
                { id: 'city', name: '城市夜景' },
            ],
            settings: defaultSettings(),
            playback: {
                activeId: null,
                status: 'idle',
                muted: true,
                volume: 0,
                pauseReasons: [],
                lastError: null,
            },
        });
        wallStore.enterBatchMode();
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/', component: LibraryView }],
        });
        await router.push('/');
        await router.isReady();
        const wrapper = mount(LibraryView, { global: { plugins: [router] } });

        expect(wrapper.get('h1').text()).toBe('批量管理');
        await wrapper.get('.wallpaper-card').trigger('click');
        await wrapper.get('[data-batch-action="add"]').trigger('click');
        await wrapper.get('[aria-label="添加到城市夜景"]').trigger('click');
        await flushPromises();

        expect(mocks.setCategoryMembership).toHaveBeenCalledWith(['ocean'], 'city', true);
    });

    it('distinguishes an empty category from a failed search', async () => {
        wallStore.search = '';
        wallStore.filter = 'all';
        wallStore.exitBatchMode();
        wallStore.activeCategoryId = 'nature';
        wallStore.applySnapshot({
            library: [
                {
                    id: 'city',
                    name: 'Neon City',
                    path: 'D:\\Wallpapers\\city.mp4',
                    kind: 'video',
                    format: 'MP4',
                    width: 1920,
                    height: 1080,
                    durationSeconds: 30,
                    thumbnailPath: null,
                    missing: false,
                    categoryIds: ['city'],
                },
            ],
            categories: [
                { id: 'nature', name: '自然风景' },
                { id: 'city', name: '城市夜景' },
            ],
            settings: defaultSettings(),
            playback: {
                activeId: null,
                status: 'idle',
                muted: true,
                volume: 0,
                pauseReasons: [],
                lastError: null,
            },
        });
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/', component: LibraryView }],
        });
        await router.push('/');
        await router.isReady();
        const wrapper = mount(LibraryView, { global: { plugins: [router] } });

        expect(wrapper.text()).toContain('此分类还没有壁纸');
        expect(wrapper.text()).not.toContain('没有搜索结果');
    });

    it('selects the confirmed span layout from the display panel', async () => {
        wallStore.search = '';
        wallStore.filter = 'all';
        wallStore.activeCategoryId = null;
        wallStore.exitBatchMode();
        wallStore.applySnapshot({
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
            displays: [display('left', -1920, true), display('right', 0, false)],
        });
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/', component: LibraryView }],
        });
        await router.push('/');
        await router.isReady();
        const wrapper = mount(LibraryView, { global: { plugins: [router] } });

        await wrapper.get('[aria-label="选择显示器"]').trigger('click');
        await wrapper.get('[data-display-mode="span"]').trigger('click');
        await wrapper.get('[data-display-action="apply"]').trigger('click');
        await flushPromises();

        expect(mocks.setDisplayLayout).toHaveBeenCalledWith('span', ['left', 'right']);
    });
});

function display(id: string, x: number, primary: boolean) {
    return {
        id,
        name: id,
        x,
        y: 0,
        width: 1920,
        height: 1080,
        primary,
        connected: true,
    };
}
