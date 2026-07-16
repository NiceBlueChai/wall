/** 验证壁纸库的空状态和媒体卡片是同一响应式数据源。 */
// @vitest-environment jsdom

import { mount } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { describe, expect, it, vi } from 'vitest';
import { defaultSettings, wallStore } from './store';
import LibraryView from './views/LibraryView.vue';

vi.mock('./api', () => ({ importMedia: vi.fn(), mediaUrl: vi.fn(() => ''), play: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

describe('LibraryView', () => {
    it('switches from the import empty state to a wallpaper card', async () => {
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/', component: LibraryView }],
        });
        await router.push('/');
        await router.isReady();
        const wrapper = mount(LibraryView, { global: { plugins: [router] } });
        expect(wrapper.text()).toContain('导入第一张壁纸');

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
        expect(wrapper.text()).not.toContain('导入第一张壁纸');
    });
});
