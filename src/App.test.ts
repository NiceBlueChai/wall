/** 验证自定义标题栏按钮调用原生窗口 API，并拥有对应 Tauri 权限。 */
// @vitest-environment jsdom

import { flushPromises, mount } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import capability from '../src-tauri/capabilities/default.json';
import App from './App.vue';
import { defaultSettings, wallStore } from './store';

const { closeMock, createCategoryMock, maximizeMock, minimizeMock } = vi.hoisted(() => ({
    closeMock: vi.fn(),
    createCategoryMock: vi.fn(),
    maximizeMock: vi.fn(),
    minimizeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/window', () => ({
    getCurrentWindow: () => ({
        close: closeMock,
        minimize: minimizeMock,
        toggleMaximize: maximizeMock,
    }),
}));
vi.mock('./api', () => ({
    bootstrap: vi.fn(),
    createCategory: createCategoryMock,
    deleteCategory: vi.fn(),
    listenToSnapshots: vi.fn(async () => () => undefined),
    renameCategory: vi.fn(),
}));

describe('App window controls', () => {
    beforeEach(() => {
        closeMock.mockReset();
        createCategoryMock.mockReset();
        maximizeMock.mockReset();
        minimizeMock.mockReset();
        wallStore.exitBatchMode();
        wallStore.activeCategoryId = null;
        wallStore.applySnapshot({
            library: [],
            categories: [],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });
    });

    it('grants and invokes every visible window action', async () => {
        expect(capability.permissions).toEqual(
            expect.arrayContaining([
                'core:window:allow-minimize',
                'core:window:allow-toggle-maximize',
                'core:window:allow-close',
            ]),
        );
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [
                { path: '/', component: { template: '<div />' } },
                { path: '/settings/:section', component: { template: '<div />' } },
            ],
        });
        await router.push('/');
        await router.isReady();
        const wrapper = mount(App, { global: { plugins: [router] } });

        expect(wrapper.get('[aria-label="最小化"] img').attributes('src')).toBe('/icons/minimize.svg');
        expect(wrapper.get('[aria-label="最大化"] img').attributes('src')).toBe('/icons/maximize.svg');
        expect(wrapper.get('[aria-label="关闭"] img').attributes('src')).toBe('/icons/close.svg');
        expect(wrapper.get('.sidebar-status i').classes()).toContain('inactive');
        await wrapper.get('[aria-label="最小化"]').trigger('click');
        await wrapper.get('[aria-label="最大化"]').trigger('click');
        await wrapper.get('[aria-label="关闭"]').trigger('click');

        expect(minimizeMock).toHaveBeenCalledOnce();
        expect(maximizeMock).toHaveBeenCalledOnce();
        expect(closeMock).toHaveBeenCalledOnce();
    });

    it('uses the confirmed category sidebar and enters batch management', async () => {
        wallStore.applySnapshot({
            library: [media('ocean', ['nature']), media('city', ['city'])],
            categories: [
                { id: 'nature', name: '自然风景' },
                { id: 'city', name: '城市夜景' },
            ],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [
                { path: '/', name: 'library', component: { template: '<div />' } },
                { path: '/settings/:section', name: 'settings', component: { template: '<div />' } },
            ],
        });
        await router.push('/');
        await router.isReady();
        const wrapper = mount(App, { global: { plugins: [router] } });

        expect(wrapper.get('.category-list').text()).toContain('自然风景');
        await wrapper.get('[data-category-id="nature"]').trigger('click');
        await wrapper.get('[aria-label="管理分类"]').trigger('click');
        await wrapper.get('[data-category-action="create"]').trigger('click');
        await wrapper.get('.category-dialog input').setValue(' 动漫收藏 ');
        await wrapper.get('.category-dialog form').trigger('submit');
        await flushPromises();
        expect(createCategoryMock).toHaveBeenCalledWith(' 动漫收藏 ');

        await wrapper.get('[aria-label="管理分类"]').trigger('click');
        await wrapper.get('[data-category-action="batch"]').trigger('click');
        expect(wallStore.activeCategoryId).toBe('nature');
        expect(wallStore.batchMode).toBe(true);
    });
});

function media(id: string, categoryIds: string[]) {
    return {
        id,
        name: id,
        path: `D:\\Wallpapers\\${id}.mp4`,
        kind: 'video' as const,
        format: 'MP4',
        width: 1920,
        height: 1080,
        durationSeconds: 30,
        thumbnailPath: null,
        missing: false,
        categoryIds,
    };
}

function idlePlayback() {
    return {
        activeId: null,
        status: 'idle' as const,
        muted: true,
        volume: 0,
        pauseReasons: [],
        lastError: null,
    };
}
