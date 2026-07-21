/** 验证详情页不会把尚未探测时长的视频误标为静态图片。 */
// @vitest-environment jsdom

import { enableAutoUnmount, flushPromises, mount } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { defaultSettings, wallStore } from './store';
import DetailView from './views/DetailView.vue';

const apiMocks = vi.hoisted(() => ({
    openMediaFolderMock: vi.fn(),
    playMock: vi.fn(),
    removeMediaMock: vi.fn(),
    setCategoryMembershipMock: vi.fn(),
    setWallpaperSettingsMock: vi.fn(),
    stopMediaMock: vi.fn(),
    stopMock: vi.fn(),
    stopTargetMock: vi.fn(),
    togglePauseMock: vi.fn(),
    toggleMediaPauseMock: vi.fn(),
    toggleTargetPauseMock: vi.fn(),
}));

vi.mock('./api', () => ({
    mediaUrl: vi.fn(() => 'asset://video'),
    openMediaFolder: apiMocks.openMediaFolderMock,
    play: apiMocks.playMock,
    relocateMedia: vi.fn(),
    removeMedia: apiMocks.removeMediaMock,
    setCategoryMembership: apiMocks.setCategoryMembershipMock,
    setWallpaperSettings: apiMocks.setWallpaperSettingsMock,
    stopMedia: apiMocks.stopMediaMock,
    stop: apiMocks.stopMock,
    stopTarget: apiMocks.stopTargetMock,
    togglePause: apiMocks.togglePauseMock,
    toggleMediaPause: apiMocks.toggleMediaPauseMock,
    toggleTargetPause: apiMocks.toggleTargetPauseMock,
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
enableAutoUnmount(afterEach);

describe('DetailView', () => {
    beforeEach(() => {
        Object.values(apiMocks).forEach((mock) => mock.mockReset());
    });

    afterEach(() => {
        vi.restoreAllMocks();
    });

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
        const wrapper = mount(DetailView, { attachTo: document.body, global: { plugins: [router] } });

        expect(wrapper.text()).toContain('时长将在播放后读取');
        expect(wrapper.text()).not.toContain('静态图片');
        expect(wrapper.text()).toContain('播放覆盖设置');
        expect(wrapper.text()).not.toContain('图片壁纸不显示帧率、硬件解码和声音设置');
        expect(wrapper.findAll('.detail-settings select')).toHaveLength(0);
        expect(wrapper.findAll('.detail-settings [role="combobox"]')).toHaveLength(3);
        expect(wrapper.get('.detail-card h2').attributes('title')).toBe('Video');
        expect(wrapper.get('.path-copy').attributes('title')).toBe('D:\\Wallpapers\\video.mp4');
        expect(wrapper.get('.detail-targets strong').attributes('title')).toBe('独立 · 显示器 1');
        const previewVideo = wrapper.get<HTMLVideoElement>('.media-preview video');
        const previewPlay = vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue();
        expect(previewVideo.attributes('controls')).toBeDefined();
        expect(previewVideo.attributes('autoplay')).toBeUndefined();
        expect(previewVideo.attributes('preload')).toBe('metadata');
        expect(previewVideo.element.muted).toBe(true);
        await wrapper.get('[data-preview-action="play"]').trigger('click');
        expect(previewPlay).toHaveBeenCalledOnce();
        await previewVideo.trigger('play');
        expect(wrapper.find('[data-preview-action="play"]').exists()).toBe(false);
        await previewVideo.trigger('pause');
        expect(wrapper.find('[data-preview-action="play"]').exists()).toBe(true);
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
        const detailScaleButtons = wrapper.findAll('.detail-setting-field .segmented button');
        expect(detailScaleButtons[0].attributes('aria-pressed')).toBe('true');
        (detailScaleButtons[0].element as HTMLElement).focus();
        await detailScaleButtons[0].trigger('keydown', { key: 'ArrowRight' });
        await wrapper.get('input[type="range"]').setValue('45');
        await flushPromises();

        expect(apiMocks.toggleMediaPauseMock).toHaveBeenCalledWith('video-1');
        expect(apiMocks.stopMediaMock).toHaveBeenCalledWith('video-1');
        expect(apiMocks.toggleTargetPauseMock).not.toHaveBeenCalled();
        expect(apiMocks.stopTargetMock).not.toHaveBeenCalled();
        expect(apiMocks.togglePauseMock).not.toHaveBeenCalled();
        expect(apiMocks.stopMock).not.toHaveBeenCalled();
        expect(apiMocks.openMediaFolderMock).toHaveBeenCalledWith('video-1');
        expect(apiMocks.setWallpaperSettingsMock).toHaveBeenCalledWith('video-1', { scaleMode: 'contain' });
        expect(document.activeElement).toBe(detailScaleButtons[1].element);
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
        expect(wrapper.get('[data-setting="aspect-ratio"]').text()).toContain('原始');
        expect(wrapper.get('[data-setting="anti-aliasing"]').text()).toContain('均衡');
        expect(wrapper.get('[data-setting="frame-rate"]').text()).toContain('60 FPS');
        const aspectRatio = wrapper.get('[data-setting="aspect-ratio"]');
        await aspectRatio.get('[role="combobox"]').trigger('click');
        await aspectRatio.get('[role="option"][data-value="ratio21x9"]').trigger('click');
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
                activeId: 'image-1',
                status: 'paused',
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
        expect(wrapper.findAll('.detail-settings select')).toHaveLength(0);
        expect(wrapper.findAll('.detail-settings [role="combobox"]')).toHaveLength(2);
        expect(wrapper.find('.mute-button').exists()).toBe(false);
        expect(wrapper.find('.volume-field').exists()).toBe(false);
        expect(wrapper.text()).toContain('播放覆盖设置');
        expect(wrapper.text()).toContain('图片壁纸不显示帧率、硬件解码和声音设置');
        expect(wrapper.findAll('.detail-actions button').map((button) => button.text())).not.toContain('暂停');
        expect(wrapper.findAll('.detail-actions button').map((button) => button.text())).not.toContain('继续');
        expect(wrapper.findAll('.detail-actions button').map((button) => button.text())).toEqual([
            '停止',
            '打开文件位置',
        ]);
        expect(wrapper.get('.playback-state').text()).toBe('正在显示');
    });

    it('locks wallpaper settings while saving and restores snapshot values after failure', async () => {
        let resolveSave!: () => void;
        const pending = new Promise<void>((resolve) => {
            resolveSave = resolve;
        });
        apiMocks.setWallpaperSettingsMock.mockReturnValue(pending);
        applyVideoSnapshot();
        const wrapper = await mountDetail();
        const scaleButtons = wrapper.findAll('.detail-setting-field .segmented button');

        await scaleButtons[1].trigger('click');
        await scaleButtons[1].trigger('click');
        await wrapper.vm.$nextTick();

        expect(apiMocks.setWallpaperSettingsMock).toHaveBeenCalledOnce();
        expect(wrapper.get('.detail-settings').attributes('aria-busy')).toBe('true');
        expect(
            wrapper
                .findAll('.detail-settings button, .detail-settings select, .detail-settings input')
                .every((control) => 'disabled' in control.attributes()),
        ).toBe(true);

        resolveSave();
        await flushPromises();
        expect(wrapper.get('.detail-settings').attributes('aria-busy')).toBe('false');

        apiMocks.setWallpaperSettingsMock.mockRejectedValueOnce(new Error('壁纸设置保存失败'));
        await scaleButtons[2].trigger('click');
        await flushPromises();

        expect(wrapper.get('.inline-error').text()).toContain('壁纸设置保存失败');
        expect(scaleButtons[0].classes()).toContain('active');
        expect(scaleButtons[2].classes()).not.toContain('active');
        expect(scaleButtons[2].attributes('disabled')).toBeUndefined();
    });

    it('prevents duplicate desktop play commands while the target is busy', async () => {
        let resolvePlay!: () => void;
        const pending = new Promise<void>((resolve) => {
            resolvePlay = resolve;
        });
        apiMocks.playMock.mockReturnValue(pending);
        applyVideoSnapshot();
        const wrapper = await mountDetail();
        const playButton = wrapper.get('[data-detail-action="play"]');

        await playButton.trigger('click');
        await playButton.trigger('click');
        await wrapper.vm.$nextTick();

        expect(apiMocks.playMock).toHaveBeenCalledOnce();
        expect(playButton.attributes()).toHaveProperty('disabled');
        expect(playButton.text()).toBe('正在设置…');

        resolvePlay();
        await flushPromises();
        expect(playButton.attributes('disabled')).toBeUndefined();
    });

    it('supports keyboard navigation and Escape for the wallpaper category menu', async () => {
        applyVideoSnapshot();
        wallStore.applySnapshot({
            ...wallStore.snapshot,
            categories: [
                { id: 'nature', name: '自然风景' },
                { id: 'city', name: '城市夜景' },
            ],
        });
        const wrapper = await mountDetail();
        const trigger = wrapper.get('[aria-label="编辑分类"]');

        await trigger.trigger('keydown', { key: 'ArrowDown' });
        await wrapper.vm.$nextTick();
        const items = wrapper.findAll('.wallpaper-category-menu [role="menuitem"]');
        expect(items).toHaveLength(2);
        expect(document.activeElement).toBe(items[0].element);

        await items[0].trigger('keydown', { key: 'End' });
        expect(document.activeElement).toBe(items[1].element);
        document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
        await wrapper.vm.$nextTick();
        expect(wrapper.find('.wallpaper-category-menu').exists()).toBe(false);
        expect(document.activeElement).toBe(trigger.element);
    });

    it('traps single-removal dialog focus and returns to its trigger after Escape', async () => {
        applyVideoSnapshot();
        const wrapper = await mountDetail();
        const trigger = wrapper.get('[data-detail-action="remove"]');

        await trigger.trigger('click');
        await wrapper.vm.$nextTick();
        const dialog = wrapper.get('[data-removal-kind="single"]');
        const close = dialog.get('.dialog-close');
        const cancel = dialog.get('[data-removal-cancel]');
        const confirm = dialog.get('[data-removal-confirm]');
        expect(document.activeElement).toBe(cancel.element);

        (confirm.element as HTMLElement).focus();
        await confirm.trigger('keydown', { key: 'Tab' });
        expect(document.activeElement).toBe(close.element);
        await close.trigger('keydown', { key: 'Tab', shiftKey: true });
        expect(document.activeElement).toBe(confirm.element);

        document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
        await wrapper.vm.$nextTick();
        expect(wrapper.find('[data-removal-kind="single"]').exists()).toBe(false);
        expect(document.activeElement).toBe(trigger.element);
        expect(wallStore.snapshot.library).toHaveLength(1);
    });

    it('keeps focus inside a busy single-removal dialog', async () => {
        let finishRemoval!: () => void;
        apiMocks.removeMediaMock.mockReturnValue(
            new Promise<void>((resolve) => {
                finishRemoval = resolve;
            }),
        );
        applyVideoSnapshot();
        const wrapper = await mountDetail();

        await wrapper.get('[data-detail-action="remove"]').trigger('click');
        const dialog = wrapper.get('[data-removal-kind="single"]');
        const removal = dialog.get('[data-removal-confirm]').trigger('click');
        await wrapper.vm.$nextTick();

        expect(wrapper.get('.page-modal-background').attributes()).toHaveProperty('inert');
        expect(document.activeElement).toBe(dialog.element);
        await dialog.trigger('keydown', { key: 'Tab' });
        expect(document.activeElement).toBe(dialog.element);

        finishRemoval();
        await removal;
        await flushPromises();
    });

    it('confirms a single library removal and warns before stopping an active target', async () => {
        wallStore.clearNotice();
        wallStore.applySnapshot({
            library: [
                {
                    id: 'video-1',
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
                activeId: 'video-1',
                status: 'playing',
                muted: true,
                volume: 0,
                pauseReasons: [],
                lastError: null,
            },
        });
        wallStore.activeCategoryId = 'nature';
        apiMocks.removeMediaMock.mockResolvedValue(undefined);
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [
                { path: '/', component: { template: '<div />' } },
                { path: '/wallpaper/:id', component: DetailView },
            ],
        });
        await router.push('/wallpaper/video-1');
        await router.isReady();
        const wrapper = mount(DetailView, { global: { plugins: [router] } });

        await wrapper.get('[data-detail-action="remove"]').trigger('click');
        const dialog = wrapper.get('[data-removal-kind="single"]');
        expect(dialog.text()).toContain('从壁纸库移除 Ocean Loop？');
        expect(dialog.text()).toContain('不会删除或修改原文件');
        expect(dialog.text()).toContain('Ocean Loop 正在使用');
        expect(dialog.get('[data-removal-cancel]').attributes()).toHaveProperty('autofocus');

        await dialog.get('[data-removal-cancel]').trigger('click');
        expect(apiMocks.removeMediaMock).not.toHaveBeenCalled();
        expect(router.currentRoute.value.path).toBe('/wallpaper/video-1');
        expect(wallStore.snapshot.library).toHaveLength(1);

        await wrapper.get('[data-detail-action="remove"]').trigger('click');
        apiMocks.removeMediaMock.mockRejectedValueOnce(new Error('移除失败'));
        await wrapper.get('[data-removal-kind="single"] [data-removal-confirm]').trigger('click');
        await flushPromises();

        expect(wrapper.get('[data-removal-kind="single"] .inline-error').text()).toBe('移除失败');
        expect(router.currentRoute.value.path).toBe('/wallpaper/video-1');

        await wrapper.get('[data-removal-kind="single"] [data-removal-confirm]').trigger('click');
        await flushPromises();

        expect(apiMocks.removeMediaMock).toHaveBeenCalledTimes(2);
        expect(apiMocks.removeMediaMock).toHaveBeenLastCalledWith('video-1');
        expect(wallStore.notice).toBe('已从壁纸库移除 Ocean Loop');
        expect(wallStore.activeCategoryId).toBeNull();
        expect(router.currentRoute.value.path).toBe('/');
        wallStore.clearNotice();
    });
});

function applyVideoSnapshot() {
    wallStore.applySnapshot({
        library: [
            {
                id: 'video-1',
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
}

async function mountDetail() {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/', component: { template: '<div />' } },
            { path: '/wallpaper/:id', component: DetailView },
        ],
    });
    await router.push('/wallpaper/video-1');
    await router.isReady();
    return mount(DetailView, { attachTo: document.body, global: { plugins: [router] } });
}
