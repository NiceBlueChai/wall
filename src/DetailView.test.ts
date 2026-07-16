/** 验证详情页不会把尚未探测时长的视频误标为静态图片。 */
// @vitest-environment jsdom

import { flushPromises, mount } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { describe, expect, it, vi } from 'vitest';
import { defaultSettings, wallStore } from './store';
import DetailView from './views/DetailView.vue';

const apiMocks = vi.hoisted(() => ({
    openMediaFolderMock: vi.fn(),
    playMock: vi.fn(),
    setScaleModeMock: vi.fn(),
    setVolumeMock: vi.fn(),
    stopMock: vi.fn(),
    togglePauseMock: vi.fn(),
}));

vi.mock('./api', () => ({
    mediaUrl: vi.fn(() => 'asset://video'),
    openMediaFolder: apiMocks.openMediaFolderMock,
    play: apiMocks.playMock,
    relocateMedia: vi.fn(),
    removeMedia: vi.fn(),
    setScaleMode: apiMocks.setScaleModeMock,
    setVolume: apiMocks.setVolumeMock,
    stop: apiMocks.stopMock,
    togglePause: apiMocks.togglePauseMock,
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
                activeId: 'video-1',
                status: 'playing',
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
        expect(wrapper.get('.media-preview video').attributes('controls')).toBeUndefined();
        expect(wrapper.get('input[type="range"]').attributes('style')).toContain('--range-progress: 0%');

        await wrapper.get('.page-heading .primary').trigger('click');
        await flushPromises();
        expect(apiMocks.playMock).toHaveBeenCalledWith('video-1');

        const actionButton = (text: string) =>
            wrapper.findAll('.detail-actions button').find((button) => button.text() === text)!;
        await actionButton('暂停').trigger('click');
        await actionButton('停止').trigger('click');
        await actionButton('打开文件位置').trigger('click');
        await wrapper.findAll('.segmented button')[1].trigger('click');
        await wrapper.get('input[type="range"]').setValue('45');
        await flushPromises();

        expect(apiMocks.togglePauseMock).toHaveBeenCalledOnce();
        expect(apiMocks.stopMock).toHaveBeenCalledOnce();
        expect(apiMocks.openMediaFolderMock).toHaveBeenCalledWith('video-1');
        expect(apiMocks.setScaleModeMock).toHaveBeenCalledWith('contain');
        expect(apiMocks.setVolumeMock).toHaveBeenCalledWith(45);
        expect(wrapper.find('.mute-button').exists()).toBe(false);

        const back = wrapper.get('.back-button');
        expect(back.attributes('aria-label')).toBe('返回壁纸库');
        expect(back.get('img').attributes('src')).toBe('/icons/back.svg');
        await back.trigger('click');
        await flushPromises();
        expect(router.currentRoute.value.path).toBe('/');
    });
});
