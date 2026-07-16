/** 验证详情页不会把尚未探测时长的视频误标为静态图片。 */
// @vitest-environment jsdom

import { mount } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { describe, expect, it, vi } from 'vitest';
import { defaultSettings, wallStore } from './store';
import DetailView from './views/DetailView.vue';

vi.mock('./api', () => ({
    mediaUrl: vi.fn(() => ''),
    openMediaFolder: vi.fn(),
    play: vi.fn(),
    relocateMedia: vi.fn(),
    removeMedia: vi.fn(),
    setMuted: vi.fn(),
    setScaleMode: vi.fn(),
    setVolume: vi.fn(),
    stop: vi.fn(),
    togglePause: vi.fn(),
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

describe('DetailView', () => {
    it('labels unknown video duration as pending', async () => {
        wallStore.applySnapshot({
            library: [
                {
                    id: 'video-1',
                    name: 'Video',
                    path: 'D:\\Wallpapers\\video.mp4',
                    kind: 'video',
                    format: 'MP4',
                    width: null,
                    height: null,
                    durationSeconds: null,
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
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/wallpaper/:id', component: DetailView }],
        });
        await router.push('/wallpaper/video-1');
        await router.isReady();
        const wrapper = mount(DetailView, { global: { plugins: [router] } });

        expect(wrapper.text()).toContain('时长将在播放后读取');
        expect(wrapper.text()).not.toContain('静态图片');
    });
});
