/** 验证壁纸库的空状态和媒体卡片是同一响应式数据源。 */
// @vitest-environment jsdom

import { flushPromises, mount } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { describe, expect, it, vi } from 'vitest';
import { defaultSettings, wallStore } from './store';
import LibraryView from './views/LibraryView.vue';

const mocks = vi.hoisted(() => ({ importMedia: vi.fn(), open: vi.fn(), play: vi.fn() }));

vi.mock('./api', () => ({ importMedia: mocks.importMedia, mediaUrl: vi.fn(() => ''), play: mocks.play }));
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
                },
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
});
