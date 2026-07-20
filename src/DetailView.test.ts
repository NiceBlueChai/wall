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
    setCategoryMembershipMock: vi.fn(),
    setWallpaperSettingsMock: vi.fn(),
    stopMock: vi.fn(),
    stopTargetMock: vi.fn(),
    togglePauseMock: vi.fn(),
    toggleTargetPauseMock: vi.fn(),
}));

vi.mock('./api', () => ({
    mediaUrl: vi.fn(() => 'asset://video'),
    openMediaFolder: apiMocks.openMediaFolderMock,
    play: apiMocks.playMock,
    relocateMedia: vi.fn(),
    removeMedia: vi.fn(),
    setCategoryMembership: apiMocks.setCategoryMembershipMock,
    setWallpaperSettings: apiMocks.setWallpaperSettingsMock,
    stop: apiMocks.stopMock,
    stopTarget: apiMocks.stopTargetMock,
    togglePause: apiMocks.togglePauseMock,
    toggleTargetPause: apiMocks.toggleTargetPauseMock,
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
                    categoryIds: ['nature'],
                },
            ],
            categories: [
                { id: 'nature', name: '自然风景' },
                { id: 'city', name: '城市夜景' },
            ],
            settings: defaultSettings(),
            playback: {
                activeId: 'other-video',
                status: 'playing',
                muted: true,
                volume: 0,
                pauseReasons: [],
                lastError: null,
                displayAssignments: [
                    {
                        targetId: 'display:primary',
                        mode: 'independent',
                        displayIds: ['primary'],
                        wallpaperId: 'video-1',
                        status: 'playing',
                        muted: true,
                        volume: 0,
                        pauseReasons: [],
                    },
                ],
            },
            displays: [
                {
                    id: 'primary',
                    name: '显示器 1',
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                    primary: true,
                    connected: true,
                },
            ],
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
        expect(wrapper.findAll('.detail-setting-field .segmented button').map((button) => button.text())).toEqual([
            '填充',
            '适应',
            '拉伸',
        ]);

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

        expect(apiMocks.toggleTargetPauseMock).toHaveBeenCalledWith('display:primary');
        expect(apiMocks.stopTargetMock).toHaveBeenCalledWith('display:primary');
        expect(apiMocks.togglePauseMock).not.toHaveBeenCalled();
        expect(apiMocks.stopMock).not.toHaveBeenCalled();
        expect(apiMocks.openMediaFolderMock).toHaveBeenCalledWith('video-1');
        expect(apiMocks.setWallpaperSettingsMock).toHaveBeenCalledWith('video-1', { scaleMode: 'contain' });
        expect(apiMocks.setWallpaperSettingsMock).toHaveBeenCalledWith('video-1', { volume: 45 });
        expect(wrapper.find('.mute-button').exists()).toBe(true);
        expect(wrapper.text()).toContain('自然风景');
        expect(wrapper.text()).toContain('当前目标');
        expect(wrapper.text()).toContain('独立 · 显示器 1');
        await wrapper.get('[aria-label="编辑分类"]').trigger('click');
        await wrapper.get('[aria-label="添加到城市夜景"]').trigger('click');
        await flushPromises();
        expect(apiMocks.setCategoryMembershipMock).toHaveBeenCalledWith(['video-1'], 'city', true);
        expect(wrapper.text()).toContain('使用全局设置');
        expect(wrapper.get('[data-setting="aspect-ratio"]').text()).toContain('21:9');
        expect(wrapper.get('[data-setting="anti-aliasing"]').text()).toContain('高质量');
        expect(wrapper.get('[data-setting="frame-rate"]').text()).toContain('源帧率');
        await wrapper.get('[data-setting="aspect-ratio"]').setValue('ratio21x9');
        await flushPromises();
        expect(apiMocks.setWallpaperSettingsMock).toHaveBeenCalledWith('video-1', {
            aspectRatio: 'ratio21x9',
        });

        const back = wrapper.get('.back-button');
        expect(back.attributes('aria-label')).toBe('返回壁纸库');
        expect(back.get('img').attributes('src')).toBe('/icons/back.svg');
        await back.trigger('click');
        await flushPromises();
        expect(router.currentRoute.value.path).toBe('/');
    });

    it('hides video-only settings for a static image', async () => {
        wallStore.applySnapshot({
            library: [
                {
                    id: 'image-1',
                    name: 'Aurora',
                    path: 'D:\\Wallpapers\\aurora.png',
                    kind: 'image',
                    format: 'PNG',
                    width: 1920,
                    height: 1080,
                    durationSeconds: null,
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
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/wallpaper/:id', component: DetailView }],
        });
        await router.push('/wallpaper/image-1');
        await router.isReady();
        const wrapper = mount(DetailView, { global: { plugins: [router] } });

        expect(wrapper.find('[data-setting="aspect-ratio"]').exists()).toBe(true);
        expect(wrapper.find('[data-setting="anti-aliasing"]').exists()).toBe(true);
        expect(wrapper.find('[data-setting="frame-rate"]').exists()).toBe(false);
        expect(wrapper.find('.mute-button').exists()).toBe(false);
        expect(wrapper.find('.volume-field').exists()).toBe(false);
    });
});
