/** 验证应用外壳的窗口操作、分类弹窗与创建后添加编排。 */
// @vitest-environment jsdom

import { enableAutoUnmount, flushPromises, mount } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { defineComponent, h, inject } from 'vue';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import capability from '../src-tauri/capabilities/default.json';
import App from './App.vue';
import { openCategoryCreatorKey } from './categoryCreator';
import { defaultSettings, wallStore } from './store';

const { closeMock, createCategoryMock, maximizeMock, minimizeMock, setCategoryMembershipMock } = vi.hoisted(() => ({
    closeMock: vi.fn(),
    createCategoryMock: vi.fn(),
    maximizeMock: vi.fn(),
    minimizeMock: vi.fn(),
    setCategoryMembershipMock: vi.fn(),
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
    setCategoryMembership: setCategoryMembershipMock,
}));
enableAutoUnmount(afterEach);

describe('App window controls', () => {
    beforeEach(() => {
        closeMock.mockReset();
        createCategoryMock.mockReset();
        maximizeMock.mockReset();
        minimizeMock.mockReset();
        setCategoryMembershipMock.mockReset();
        wallStore.exitBatchMode();
        wallStore.activeCategoryId = null;
        wallStore.clearNotice();
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

    it('keeps category management focused on category actions', async () => {
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
        expect(wallStore.activeCategoryId).toBe('nature');
        expect(wrapper.find('[data-category-action="batch"]').exists()).toBe(false);
        expect(wrapper.find('[data-category-action="rename"]').exists()).toBe(true);
        expect(wrapper.find('[data-category-action="delete"]').exists()).toBe(true);
    });

    it('creates a category from a route surface and assigns it to the requested wallpaper', async () => {
        wallStore.applySnapshot({
            library: [media('video-1', [])],
            categories: [],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });
        const createdSnapshot = {
            ...wallStore.snapshot,
            categories: [{ id: 'created', name: '动漫收藏' }],
        };
        createCategoryMock.mockResolvedValue(createdSnapshot);
        setCategoryMembershipMock.mockResolvedValue(createdSnapshot);
        const wrapper = await mountAppWithCategoryProbe();
        const trigger = wrapper.get('[data-open-category-creator]');

        await trigger.trigger('click');
        await wrapper.vm.$nextTick();
        expect(wrapper.get('#category-dialog-title').text()).toBe('新建分类');
        expect(wrapper.get('.category-dialog button[type="submit"]').text()).toBe('创建并添加');

        await wrapper.get('.category-dialog input').setValue('动漫收藏');
        await wrapper.get('.category-dialog form').trigger('submit');
        await flushPromises();

        expect(createCategoryMock).toHaveBeenCalledWith('动漫收藏');
        expect(setCategoryMembershipMock).toHaveBeenCalledWith(['video-1'], 'created', true);
        expect(wrapper.find('.category-dialog').exists()).toBe(false);
        expect(document.activeElement).toBe(trigger.element);
    });

    it('keeps a created category and reports when automatic assignment fails', async () => {
        wallStore.applySnapshot({
            library: [media('video-1', [])],
            categories: [],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });
        createCategoryMock.mockResolvedValue({
            ...wallStore.snapshot,
            categories: [{ id: 'created', name: '动漫收藏' }],
        });
        setCategoryMembershipMock.mockRejectedValue(new Error('添加失败'));
        const wrapper = await mountAppWithCategoryProbe();
        const trigger = wrapper.get('[data-open-category-creator]');

        await trigger.trigger('click');
        await wrapper.get('.category-dialog input').setValue('动漫收藏');
        await wrapper.get('.category-dialog form').trigger('submit');
        await flushPromises();

        expect(createCategoryMock).toHaveBeenCalledOnce();
        expect(setCategoryMembershipMock).toHaveBeenCalledWith(['video-1'], 'created', true);
        expect(wrapper.find('.category-dialog').exists()).toBe(false);
        expect(wrapper.get('.error-toast').text()).toBe('分类已创建，但添加到当前壁纸失败，请重试');
        expect(document.activeElement).toBe(trigger.element);
    });

    it('supports keyboard navigation and dismissal for the category menu', async () => {
        wallStore.applySnapshot({
            library: [media('ocean', ['nature'])],
            categories: [{ id: 'nature', name: '自然风景' }],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });
        wallStore.activeCategoryId = 'nature';
        const wrapper = await mountApp();
        const trigger = wrapper.get('[aria-label="管理分类"]');

        await trigger.trigger('keydown', { key: 'ArrowDown' });
        await wrapper.vm.$nextTick();
        const items = wrapper.findAll('.category-action-menu [role="menuitem"]');
        expect(items).toHaveLength(3);
        expect(document.activeElement).toBe(items[0].element);

        await items[0].trigger('keydown', { key: 'End' });
        expect(document.activeElement).toBe(items[2].element);
        await items[2].trigger('keydown', { key: 'ArrowDown' });
        expect(document.activeElement).toBe(items[0].element);

        document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
        await wrapper.vm.$nextTick();
        expect(wrapper.find('.category-action-menu').exists()).toBe(false);
        expect(document.activeElement).toBe(trigger.element);

        await trigger.trigger('click');
        const outside = wrapper.get('[aria-label="添加分类"]');
        (outside.element as HTMLElement).focus();
        outside.element.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true }));
        await wrapper.vm.$nextTick();
        expect(wrapper.find('.category-action-menu').exists()).toBe(false);
        expect(document.activeElement).toBe(trigger.element);
    });

    it('locks category submission, traps focus and restores the dialog trigger', async () => {
        let resolveCreate!: () => void;
        const pending = new Promise<void>((resolve) => {
            resolveCreate = resolve;
        });
        createCategoryMock.mockReturnValue(pending);
        const wrapper = await mountApp();
        const trigger = wrapper.get('[aria-label="添加分类"]');

        await trigger.trigger('click');
        await wrapper.vm.$nextTick();
        const dialog = wrapper.get('.category-dialog');
        const input = dialog.get('input');
        expect(dialog.attributes('role')).toBe('dialog');
        expect(dialog.attributes('aria-modal')).toBe('true');
        expect(dialog.attributes('tabindex')).toBe('-1');
        expect(wrapper.get('.titlebar').attributes()).toHaveProperty('inert');
        expect(wrapper.get('.body-shell').attributes()).toHaveProperty('inert');
        expect(document.activeElement).toBe(input.element);

        const submit = dialog.get('button[type="submit"]');
        (submit.element as HTMLElement).focus();
        await submit.trigger('keydown', { key: 'Tab' });
        expect(document.activeElement).toBe(input.element);
        await input.trigger('keydown', { key: 'Tab', shiftKey: true });
        expect(document.activeElement).toBe(submit.element);

        await input.setValue('动漫收藏');
        await dialog.get('form').trigger('submit');
        await dialog.get('form').trigger('submit');
        await wrapper.vm.$nextTick();
        expect(createCategoryMock).toHaveBeenCalledOnce();
        expect(dialog.get('[data-category-cancel]').attributes()).toHaveProperty('disabled');
        expect(input.attributes()).toHaveProperty('disabled');
        expect(document.activeElement).toBe(dialog.element);
        await dialog.trigger('keydown', { key: 'Tab' });
        expect(document.activeElement).toBe(dialog.element);

        document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
        await wrapper.vm.$nextTick();
        expect(wrapper.find('.category-dialog').exists()).toBe(true);

        resolveCreate();
        await flushPromises();
        expect(wrapper.find('.category-dialog').exists()).toBe(false);
        expect(wrapper.get('.titlebar').attributes('inert')).toBeUndefined();
        expect(wrapper.get('.body-shell').attributes('inert')).toBeUndefined();
        expect(document.activeElement).toBe(trigger.element);
    });

    it('counts non-BMP category names by Unicode code point at the 40-character boundary', async () => {
        const wrapper = await mountApp();

        await wrapper.get('[aria-label="添加分类"]').trigger('click');
        const input = wrapper.get('.category-dialog input');
        expect(input.attributes('maxlength')).toBeUndefined();
        await input.setValue('😀'.repeat(40));
        await wrapper.get('.category-dialog form').trigger('submit');
        await flushPromises();

        expect(createCategoryMock).toHaveBeenCalledWith('😀'.repeat(40));

        await wrapper.get('[aria-label="添加分类"]').trigger('click');
        await wrapper.get('.category-dialog input').setValue('😀'.repeat(41));
        await wrapper.get('.category-dialog form').trigger('submit');
        await flushPromises();

        expect(createCategoryMock).toHaveBeenCalledOnce();
        expect(wrapper.get('.category-dialog .inline-error').text()).toBe('分类名称最多 40 个字符');
    });

    it('renders shared success notices through the existing toast surface', async () => {
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

        wallStore.showNotice('已从壁纸库移除');
        await wrapper.vm.$nextTick();

        const notice = wrapper.get('.success-toast');
        expect(notice.text()).toBe('已从壁纸库移除');
        expect(notice.attributes('role')).toBe('status');
        expect(notice.attributes('aria-live')).toBe('polite');
        wrapper.unmount();
        wallStore.clearNotice();
    });

    it('summarizes multiple display targets across running, paused and error states', async () => {
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/', name: 'library', component: { template: '<div />' } }],
        });
        await router.push('/');
        await router.isReady();
        wallStore.applySnapshot({
            library: [media('ocean', []), media('city', [])],
            categories: [],
            settings: defaultSettings(),
            playback: playbackWithStatuses('playing', 'playing'),
            displays: [display('left', true), display('right', false)],
        });
        const wrapper = mount(App, { global: { plugins: [router] } });

        expect(wrapper.get('.sidebar-status').text()).toBe('2 个显示目标 · 运行中');

        wallStore.applySnapshot({
            ...wallStore.snapshot,
            playback: playbackWithStatuses('playing', 'paused'),
        });
        await wrapper.vm.$nextTick();
        expect(wrapper.get('.sidebar-status').text()).toBe('2 个显示目标 · 部分暂停');

        wallStore.applySnapshot({
            ...wallStore.snapshot,
            playback: playbackWithStatuses('paused', 'paused'),
        });
        await wrapper.vm.$nextTick();
        expect(wrapper.get('.sidebar-status').text()).toBe('2 个显示目标 · 全部暂停');

        wallStore.applySnapshot({
            ...wallStore.snapshot,
            playback: playbackWithStatuses('playing', 'error'),
        });
        await wrapper.vm.$nextTick();
        expect(wrapper.get('.sidebar-status').text()).toBe('2 个显示目标 · 错误');
        expect(wrapper.get('.sidebar-status i').classes()).toContain('error');
    });

    it('suppresses the browser context menu outside editable controls', async () => {
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

        const shellEvent = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
        wrapper.get('.sidebar').element.dispatchEvent(shellEvent);
        expect(shellEvent.defaultPrevented).toBe(true);

        await wrapper.get('[aria-label="添加分类"]').trigger('click');
        const inputEvent = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
        wrapper.get('.category-dialog input').element.dispatchEvent(inputEvent);
        expect(inputEvent.defaultPrevented).toBe(false);
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

async function mountApp() {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/', name: 'library', component: { template: '<div />' } },
            { path: '/settings/:section', name: 'settings', component: { template: '<div />' } },
        ],
    });
    await router.push('/');
    await router.isReady();
    return mount(App, { attachTo: document.body, global: { plugins: [router] } });
}

const CategoryCreatorProbe = defineComponent({
    setup() {
        const openCategoryCreator = inject(openCategoryCreatorKey);
        if (!openCategoryCreator) throw new Error('缺少分类创建入口');
        return () =>
            h(
                'button',
                {
                    'data-open-category-creator': '',
                    onClick: (event: MouseEvent) =>
                        openCategoryCreator(['video-1'], event.currentTarget as HTMLElement),
                },
                '新建并添加',
            );
    },
});

async function mountAppWithCategoryProbe() {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [{ path: '/', name: 'library', component: CategoryCreatorProbe }],
    });
    await router.push('/');
    await router.isReady();
    return mount(App, { attachTo: document.body, global: { plugins: [router] } });
}

function playbackWithStatuses(first: 'playing' | 'paused' | 'error', second: 'playing' | 'paused' | 'error') {
    return {
        ...idlePlayback(),
        activeId: 'ocean',
        status: first,
        displayAssignments: [
            displayAssignment('display:left', 'left', 'ocean', first),
            displayAssignment('display:right', 'right', 'city', second),
        ],
    };
}

function displayAssignment(
    targetId: string,
    displayId: string,
    wallpaperId: string,
    status: 'playing' | 'paused' | 'error',
) {
    return {
        targetId,
        mode: 'independent' as const,
        displayIds: [displayId],
        wallpaperId,
        status,
        muted: true,
        volume: 0,
        pauseReasons: status === 'paused' ? ['manual' as const] : [],
    };
}

function display(id: string, primary: boolean) {
    return {
        id,
        name: id,
        x: primary ? 0 : 1920,
        y: 0,
        width: 1920,
        height: 1080,
        primary,
        connected: true,
    };
}
