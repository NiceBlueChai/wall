/** 验证壁纸库的空状态和媒体卡片是同一响应式数据源。 */
// @vitest-environment jsdom

import { enableAutoUnmount, flushPromises, mount } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { defaultSettings, wallStore } from './store';
import LibraryView from './views/LibraryView.vue';

const mocks = vi.hoisted(() => ({
    importMedia: vi.fn(),
    open: vi.fn(),
    play: vi.fn(),
    removeMediaBatch: vi.fn(),
    removeMissingMedia: vi.fn(),
    scanLibrary: vi.fn(),
    setCategoryMembership: vi.fn(),
    setDisplayLayout: vi.fn(),
}));

vi.mock('./api', () => ({
    importMedia: mocks.importMedia,
    mediaUrl: vi.fn(() => ''),
    play: mocks.play,
    removeMediaBatch: mocks.removeMediaBatch,
    removeMissingMedia: mocks.removeMissingMedia,
    scanLibrary: mocks.scanLibrary,
    setCategoryMembership: mocks.setCategoryMembership,
    setDisplayLayout: mocks.setDisplayLayout,
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: mocks.open }));
enableAutoUnmount(afterEach);

describe('LibraryView', () => {
    beforeEach(() => {
        Object.values(mocks).forEach((mock) => mock.mockReset());
        wallStore.search = '';
        wallStore.filter = 'all';
        wallStore.activeCategoryId = null;
        wallStore.exitBatchMode();
        wallStore.clearNotice();
        wallStore.applySnapshot({
            library: [],
            categories: [],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });
    });

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
        expect(mocks.play).not.toHaveBeenCalled();

        const quickPlay = wrapper.get('[data-card-action="quick-play"]');
        expect(quickPlay.attributes('aria-label')).toBe('将 Ocean Loop 设为壁纸');
        await quickPlay.trigger('click');
        await flushPromises();
        expect(mocks.play).toHaveBeenCalledWith('ocean');
        expect(router.currentRoute.value.path).toBe('/');

        mocks.open.mockResolvedValueOnce('D:\\Wallpapers\\new.mp4');
        mocks.importMedia.mockResolvedValueOnce(snapshotWithLibrary([media('new', false)]));
        await wrapper.get('.heading-actions .primary').trigger('click');
        await flushPromises();
        expect(mocks.importMedia).toHaveBeenCalledWith(['D:\\Wallpapers\\new.mp4']);

        wallStore.search = 'not-found';
        await wrapper.vm.$nextTick();
        expect(wrapper.text()).toContain('没有搜索结果');
        expect(wrapper.get('.compact-empty img').attributes('src')).toBe('/icons/search.svg');
    });

    it('shows import progress, prevents duplicate imports and restores focus with a success notice', async () => {
        let finishImport!: () => void;
        mocks.open.mockResolvedValue('D:\\Wallpapers\\new.mp4');
        mocks.importMedia.mockReturnValue(
            new Promise((resolve) => {
                finishImport = () => resolve(snapshotWithLibrary([media('new', false)]));
            }),
        );
        const wrapper = await mountLibrary();
        const trigger = wrapper.get('[data-library-action="import"]');
        (trigger.element as HTMLElement).focus();

        const importAction = trigger.trigger('click');
        await flushPromises();

        const importDialog = wrapper.get('[aria-label="正在导入壁纸"]');
        expect(importDialog.attributes('tabindex')).toBe('-1');
        expect(wrapper.get('.page-modal-background').attributes()).toHaveProperty('inert');
        expect(document.activeElement).toBe(importDialog.element);
        await importDialog.trigger('keydown', { key: 'Tab' });
        expect(document.activeElement).toBe(importDialog.element);
        expect(trigger.attributes()).toHaveProperty('disabled');
        expect(trigger.text()).toBe('正在导入…');
        await trigger.trigger('click');
        expect(mocks.open).toHaveBeenCalledOnce();
        expect(mocks.importMedia).toHaveBeenCalledOnce();

        finishImport();
        await importAction;
        await flushPromises();

        expect(wrapper.find('[aria-label="正在导入壁纸"]').exists()).toBe(false);
        expect(wrapper.get('.page-modal-background').attributes('inert')).toBeUndefined();
        expect(wallStore.notice).toBe('已导入 1 项壁纸');
        expect(document.activeElement).toBe(trigger.element);
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

    it('hides card quick play for missing items and while batch management is active', async () => {
        wallStore.applySnapshot({
            library: [media('ready', false), media('missing', true)],
            categories: [],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });
        const wrapper = await mountLibrary();

        expect(wrapper.findAll('[data-card-action="quick-play"]')).toHaveLength(1);
        expect(wrapper.get('[data-card-action="quick-play"]').attributes('aria-label')).toBe('将 ready 设为壁纸');

        wallStore.enterBatchMode();
        await wrapper.vm.$nextTick();

        expect(wrapper.find('[data-card-action="quick-play"]').exists()).toBe(false);
    });

    it('prevents duplicate quick-play and import requests while each action is pending', async () => {
        let resolvePlay!: () => void;
        const pendingPlay = new Promise<void>((resolve) => {
            resolvePlay = resolve;
        });
        mocks.play.mockReturnValue(pendingPlay);
        wallStore.applySnapshot({
            library: [media('ready', false)],
            categories: [],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });
        const wrapper = await mountLibrary();
        const quickPlay = wrapper.get('[data-card-action="quick-play"]');

        await quickPlay.trigger('click');
        await quickPlay.trigger('click');
        await wrapper.vm.$nextTick();
        expect(mocks.play).toHaveBeenCalledOnce();
        expect(quickPlay.attributes()).toHaveProperty('disabled');
        expect(quickPlay.attributes('aria-busy')).toBe('true');

        resolvePlay();
        await flushPromises();
        expect(quickPlay.attributes('disabled')).toBeUndefined();

        let resolveOpen!: (value: null) => void;
        const pendingOpen = new Promise<null>((resolve) => {
            resolveOpen = resolve;
        });
        mocks.open.mockReturnValue(pendingOpen);
        const importButton = wrapper.get('[data-library-action="import"]');
        await importButton.trigger('click');
        await importButton.trigger('click');
        await wrapper.vm.$nextTick();
        expect(mocks.open).toHaveBeenCalledOnce();
        expect(importButton.attributes()).toHaveProperty('disabled');

        resolveOpen(null);
        await flushPromises();
        expect(importButton.attributes('disabled')).toBeUndefined();
    });

    it('marks every wallpaper assigned to a display target as active', async () => {
        wallStore.applySnapshot({
            library: [media('ocean', false), media('city', false)],
            categories: [],
            settings: defaultSettings(),
            playback: {
                ...idlePlayback(),
                activeId: 'ocean',
                status: 'playing',
                displayAssignments: [displayAssignment('left', 'ocean'), displayAssignment('right', 'city')],
            },
        });
        const wrapper = await mountLibrary();

        expect(wrapper.findAll('.wallpaper-card.active')).toHaveLength(2);
        expect(wrapper.findAll('.card-copy small').map((node) => node.text())).toEqual([
            'VIDEO · 正在运行',
            'VIDEO · 正在运行',
        ]);
    });

    it('locks batch context while updating category membership and restores focus after success', async () => {
        let resolveCategory!: () => void;
        const pending = new Promise<void>((resolve) => {
            resolveCategory = resolve;
        });
        mocks.setCategoryMembership.mockReturnValue(pending);
        wallStore.applySnapshot({
            library: [media('ready', false)],
            categories: [{ id: 'city', name: '城市夜景' }],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });
        wallStore.enterBatchMode();
        const wrapper = await mountLibrary();
        await wrapper.get('.wallpaper-card').trigger('click');
        const trigger = wrapper.get('[data-batch-action="add"]');
        await trigger.trigger('click');
        const category = wrapper.get('[aria-label="添加到城市夜景"]');

        await category.trigger('click');
        await category.trigger('click');
        await wrapper.vm.$nextTick();
        expect(mocks.setCategoryMembership).toHaveBeenCalledOnce();
        expect(wrapper.get('[data-batch-action="cancel"]').attributes()).toHaveProperty('disabled');
        expect(category.attributes()).toHaveProperty('disabled');

        resolveCategory();
        await flushPromises();
        expect(wrapper.find('.batch-category-menu').exists()).toBe(false);
        expect(document.activeElement).toBe(trigger.element);
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
        const wrapper = mount(LibraryView, { attachTo: document.body, global: { plugins: [router] } });

        await wrapper.get('[aria-label="选择显示器"]').trigger('click');
        const independent = wrapper.get('[data-display-mode="independent"]');
        const clone = wrapper.get('[data-display-mode="clone"]');
        const span = wrapper.get('[data-display-mode="span"]');
        expect(independent.attributes('aria-pressed')).toBe('true');
        (independent.element as HTMLElement).focus();
        await independent.trigger('keydown', { key: 'ArrowRight' });
        expect(document.activeElement).toBe(clone.element);
        await clone.trigger('keydown', { key: 'ArrowRight' });
        expect(document.activeElement).toBe(span.element);
        expect(span.attributes('aria-pressed')).toBe('true');
        await wrapper.get('[data-display-action="apply"]').trigger('click');
        await flushPromises();

        expect(mocks.setDisplayLayout).toHaveBeenCalledWith('span', ['left', 'right']);
    });

    it('keeps the display draft open after a failed apply and closes only after retry succeeds', async () => {
        wallStore.applySnapshot({
            library: [],
            categories: [],
            settings: defaultSettings(),
            playback: idlePlayback(),
            displays: [display('left', -1920, true), display('right', 0, false)],
        });
        mocks.setDisplayLayout.mockRejectedValueOnce(new Error('无法应用显示布局')).mockResolvedValueOnce(undefined);
        const wrapper = await mountLibrary();

        await wrapper.get('[aria-label="选择显示器"]').trigger('click');
        await wrapper.get('[data-display-mode="span"]').trigger('click');
        await wrapper.get('[data-display-action="apply"]').trigger('click');
        await flushPromises();

        expect(wrapper.find('.display-selector-panel').exists()).toBe(true);
        expect(wrapper.get('[data-display-mode="span"]').classes()).toContain('active');
        expect(wrapper.findAll('.monitor-card.selected')).toHaveLength(2);
        expect(wrapper.get('.display-selector-panel .inline-error').text()).toBe('无法应用显示布局');

        await wrapper.get('[data-display-action="apply"]').trigger('click');
        await flushPromises();

        expect(mocks.setDisplayLayout).toHaveBeenCalledTimes(2);
        expect(wrapper.find('.display-selector-panel').exists()).toBe(false);
    });

    it('locks competing top-level actions while applying a display draft', async () => {
        let resolveLayout!: () => void;
        mocks.setDisplayLayout.mockReturnValue(
            new Promise<void>((resolve) => {
                resolveLayout = resolve;
            }),
        );
        wallStore.applySnapshot({
            library: [],
            categories: [],
            settings: defaultSettings(),
            playback: idlePlayback(),
            displays: [display('left', -1920, true), display('right', 0, false)],
        });
        const wrapper = await mountLibrary();

        await wrapper.get('[aria-label="选择显示器"]').trigger('click');
        await wrapper.get('[data-display-mode="span"]').trigger('click');
        const apply = wrapper.get('[data-display-action="apply"]').trigger('click');
        await wrapper.vm.$nextTick();

        const management = wrapper.get('[aria-label="管理壁纸库"]');
        const importing = wrapper.get('[data-library-action="import"]');
        expect(management.attributes()).toHaveProperty('disabled');
        expect(importing.attributes()).toHaveProperty('disabled');
        await management.trigger('click');
        expect(wrapper.find('.library-management-menu').exists()).toBe(false);
        expect(wrapper.find('.display-selector-panel').exists()).toBe(true);

        resolveLayout();
        await apply;
        await flushPromises();
        expect(wrapper.find('.display-selector-panel').exists()).toBe(false);
    });

    it('rejects offline and duplicate display drafts before applying them', async () => {
        wallStore.applySnapshot({
            library: [],
            categories: [],
            settings: {
                ...defaultSettings(),
                displayMode: 'span',
                selectedDisplayIds: ['left', 'offline'],
            },
            playback: idlePlayback(),
            displays: [display('left', -1920, true), display('offline', 0, false, false)],
        });
        const wrapper = await mountLibrary();

        await wrapper.get('[aria-label="选择显示器"]').trigger('click');
        expect(wrapper.get('[data-display-action="apply"]').attributes()).toHaveProperty('disabled');
        expect(wrapper.text()).toContain('连接至少两块在线屏幕后可使用复制和铺展');

        await wrapper.get('[data-display-action="cancel"]').trigger('click');
        wallStore.applySnapshot({
            ...wallStore.snapshot,
            settings: {
                ...wallStore.snapshot.settings,
                selectedDisplayIds: ['left', 'left'],
            },
            displays: [display('left', -1920, true), display('right', 0, false)],
        });
        await wrapper.get('[aria-label="选择显示器"]').trigger('click');

        expect(wrapper.get('[data-display-action="apply"]').attributes()).toHaveProperty('disabled');
        expect(mocks.setDisplayLayout).not.toHaveBeenCalled();
    });

    it('opens batch management from the library management menu', async () => {
        const wrapper = await mountLibrary();

        await wrapper.get('[aria-label="选择显示器"]').trigger('click');
        expect(wrapper.find('.display-selector-panel').exists()).toBe(true);
        await wrapper.get('[aria-label="管理壁纸库"]').trigger('click');
        expect(wrapper.find('.display-selector-panel').exists()).toBe(false);
        expect(wrapper.get('[data-library-action="batch"]').text()).toContain('批量管理');
        expect(wrapper.get('[data-library-action="cleanup"]').text()).toContain('清理失效项');

        await wrapper.get('[data-library-action="batch"]').trigger('click');
        expect(wallStore.batchMode).toBe(true);
        expect(wrapper.get('h1').text()).toBe('批量管理');
    });

    it('supports keyboard navigation, Escape and outside dismissal for the management menu', async () => {
        const wrapper = await mountLibrary();
        const trigger = wrapper.get('[aria-label="管理壁纸库"]');

        await trigger.trigger('keydown', { key: 'ArrowDown' });
        await wrapper.vm.$nextTick();
        const items = wrapper.findAll('[role="menuitem"]');
        expect(items).toHaveLength(2);
        expect(document.activeElement).toBe(items[0].element);

        await items[0].trigger('keydown', { key: 'ArrowDown' });
        expect(document.activeElement).toBe(items[1].element);
        await items[1].trigger('keydown', { key: 'Home' });
        expect(document.activeElement).toBe(items[0].element);
        await items[0].trigger('keydown', { key: 'End' });
        expect(document.activeElement).toBe(items[1].element);

        document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
        await wrapper.vm.$nextTick();
        expect(wrapper.find('.library-management-menu').exists()).toBe(false);
        expect(document.activeElement).toBe(trigger.element);

        await trigger.trigger('click');
        const outside = wrapper.get('[data-library-action="import"]');
        (outside.element as HTMLElement).focus();
        outside.element.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true }));
        await wrapper.vm.$nextTick();
        expect(wrapper.find('.library-management-menu').exists()).toBe(false);
        expect(document.activeElement).toBe(trigger.element);
    });

    it('cancels the display draft with Escape or outside click and restores trigger focus', async () => {
        wallStore.applySnapshot({
            library: [],
            categories: [],
            settings: defaultSettings(),
            playback: idlePlayback(),
            displays: [display('left', -1920, true), display('right', 0, false)],
        });
        const wrapper = await mountLibrary();
        const trigger = wrapper.get('[aria-label="选择显示器"]');

        await trigger.trigger('click');
        await wrapper.get('[data-display-mode="span"]').trigger('click');
        document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
        await wrapper.vm.$nextTick();
        expect(wrapper.find('.display-selector-panel').exists()).toBe(false);
        expect(document.activeElement).toBe(trigger.element);

        await trigger.trigger('click');
        const outside = wrapper.get('[data-library-action="import"]');
        (outside.element as HTMLElement).focus();
        outside.element.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true }));
        await wrapper.vm.$nextTick();
        expect(wrapper.find('.display-selector-panel').exists()).toBe(false);
        expect(document.activeElement).toBe(trigger.element);
        expect(mocks.setDisplayLayout).not.toHaveBeenCalled();
    });

    it('rescans before confirming cleanup and preserves state when cancelled', async () => {
        wallStore.applySnapshot({
            library: [media('missing-active', true), media('missing-idle', true), media('valid', false)],
            categories: [],
            settings: defaultSettings(),
            playback: activePlayback('missing-active'),
        });
        mocks.scanLibrary.mockResolvedValue(undefined);
        mocks.removeMissingMedia.mockImplementation(async () => {
            const [restored, , valid] = wallStore.snapshot.library;
            wallStore.applySnapshot({
                ...wallStore.snapshot,
                library: [{ ...restored, missing: false }, valid],
            });
            return wallStore.snapshot;
        });
        const wrapper = await mountLibrary();

        await wrapper.get('[aria-label="管理壁纸库"]').trigger('click');
        await wrapper.get('[data-library-action="cleanup"]').trigger('click');
        await flushPromises();

        expect(mocks.scanLibrary).toHaveBeenCalledOnce();
        expect(wrapper.get('[data-removal-kind="cleanup"]').text()).toContain('清理 2 个失效项？');
        expect(wrapper.get('[data-removal-kind="cleanup"]').text()).toContain('源文件不会被删除或修改');
        expect(wrapper.get('[data-removal-kind="cleanup"]').text()).toContain('其中 1 项仍分配到屏幕');
        expect(wrapper.get('[data-removal-kind="cleanup"] [data-removal-cancel]').attributes()).toHaveProperty(
            'autofocus',
        );

        await wrapper.get('[data-removal-kind="cleanup"] [data-removal-cancel]').trigger('click');
        expect(mocks.removeMissingMedia).not.toHaveBeenCalled();
        expect(wallStore.snapshot.library).toHaveLength(3);

        await wrapper.get('[aria-label="管理壁纸库"]').trigger('click');
        await wrapper.get('[data-library-action="cleanup"]').trigger('click');
        await flushPromises();
        await wrapper.get('[data-removal-kind="cleanup"] [data-removal-confirm]').trigger('click');
        await flushPromises();

        expect(mocks.removeMissingMedia).toHaveBeenCalledOnce();
        expect(wallStore.notice).toBe('已清理 1 个失效项');
    });

    it('shows an explicit busy state while rescanning missing files', async () => {
        let finishScan!: () => void;
        mocks.scanLibrary.mockReturnValue(
            new Promise<void>((resolve) => {
                finishScan = resolve;
            }),
        );
        wallStore.applySnapshot({
            library: [media('missing', true)],
            categories: [],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });
        const wrapper = await mountLibrary();
        const management = wrapper.get('[aria-label="管理壁纸库"]');

        await management.trigger('click');
        const scan = wrapper.get('[data-library-action="cleanup"]').trigger('click');
        await wrapper.vm.$nextTick();

        expect(management.attributes()).toHaveProperty('disabled');
        expect(management.attributes('aria-busy')).toBe('true');
        expect(management.text()).toContain('正在扫描…');

        finishScan();
        await scan;
        await flushPromises();
        expect(wrapper.find('[data-removal-kind="cleanup"]').exists()).toBe(true);
    });

    it('reports an up-to-date library when cleanup finds nothing', async () => {
        wallStore.applySnapshot({
            library: [media('valid', false)],
            categories: [],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });
        mocks.scanLibrary.mockResolvedValue(undefined);
        const wrapper = await mountLibrary();

        await wrapper.get('[aria-label="管理壁纸库"]').trigger('click');
        await wrapper.get('[data-library-action="cleanup"]').trigger('click');
        await flushPromises();

        expect(mocks.scanLibrary).toHaveBeenCalledOnce();
        expect(wrapper.find('[data-removal-kind="cleanup"]').exists()).toBe(false);
        expect(wallStore.notice).toBe('没有发现失效项');
    });

    it('confirms a lightweight batch removal without changing selection on cancel', async () => {
        wallStore.applySnapshot({
            library: [media('active', false), media('idle', false)],
            categories: [],
            settings: defaultSettings(),
            playback: activePlayback('active'),
        });
        wallStore.enterBatchMode();
        mocks.removeMediaBatch.mockResolvedValue(undefined);
        const wrapper = await mountLibrary();

        await wrapper.findAll('.wallpaper-card')[0].trigger('click');
        await wrapper.findAll('.wallpaper-card')[1].trigger('click');
        await wrapper.get('[data-batch-action="remove-library"]').trigger('click');

        const dialog = wrapper.get('[data-removal-kind="batch"]');
        expect(dialog.text()).toContain('从壁纸库移除 2 项？');
        expect(dialog.text()).toContain('不会删除或修改任何源文件');
        expect(dialog.text()).toContain('其中 1 项仍分配到屏幕');
        expect(dialog.get('[data-removal-cancel]').attributes()).toHaveProperty('autofocus');

        await dialog.get('[data-removal-cancel]').trigger('click');
        expect(wallStore.selectedMediaIds).toEqual(['active', 'idle']);
        expect(mocks.removeMediaBatch).not.toHaveBeenCalled();

        await wrapper.get('[data-batch-action="remove-library"]').trigger('click');
        mocks.removeMediaBatch.mockRejectedValueOnce(new Error('移除失败'));
        await wrapper.get('[data-removal-kind="batch"] [data-removal-confirm]').trigger('click');
        await flushPromises();

        expect(wrapper.get('[data-removal-kind="batch"] .inline-error').text()).toBe('移除失败');
        expect(wallStore.selectedMediaIds).toEqual(['active', 'idle']);

        await wrapper.get('[data-removal-kind="batch"] [data-removal-confirm]').trigger('click');
        await flushPromises();

        expect(mocks.removeMediaBatch).toHaveBeenCalledTimes(2);
        expect(mocks.removeMediaBatch).toHaveBeenLastCalledWith(['active', 'idle']);
        expect(wallStore.notice).toBe('已从壁纸库移除 2 项');
        expect(wallStore.batchMode).toBe(false);
        expect(wallStore.selectedMediaIds).toEqual([]);
    });

    it('traps dialog focus and returns it to the batch removal action after Escape', async () => {
        wallStore.applySnapshot({
            library: [media('ready', false)],
            categories: [],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });
        wallStore.enterBatchMode();
        const wrapper = await mountLibrary();

        await wrapper.get('.wallpaper-card').trigger('click');
        const trigger = wrapper.get('[data-batch-action="remove-library"]');
        await trigger.trigger('click');
        await wrapper.vm.$nextTick();

        const dialog = wrapper.get('[data-removal-kind="batch"]');
        const cancel = dialog.get('[data-removal-cancel]');
        const confirm = dialog.get('[data-removal-confirm]');
        const close = dialog.get('.dialog-close');
        expect(document.activeElement).toBe(cancel.element);

        (confirm.element as HTMLElement).focus();
        await confirm.trigger('keydown', { key: 'Tab' });
        expect(document.activeElement).toBe(close.element);
        await close.trigger('keydown', { key: 'Tab', shiftKey: true });
        expect(document.activeElement).toBe(confirm.element);

        document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
        await wrapper.vm.$nextTick();
        expect(wrapper.find('[data-removal-kind="batch"]').exists()).toBe(false);
        expect(document.activeElement).toBe(trigger.element);
        expect(wallStore.selectedMediaIds).toEqual(['ready']);
    });

    it('keeps focus inside a busy batch-removal dialog', async () => {
        let finishRemoval!: () => void;
        mocks.removeMediaBatch.mockReturnValue(
            new Promise<void>((resolve) => {
                finishRemoval = resolve;
            }),
        );
        wallStore.applySnapshot({
            library: [media('ready', false)],
            categories: [],
            settings: defaultSettings(),
            playback: idlePlayback(),
        });
        wallStore.enterBatchMode();
        const wrapper = await mountLibrary();

        await wrapper.get('.wallpaper-card').trigger('click');
        await wrapper.get('[data-batch-action="remove-library"]').trigger('click');
        const dialog = wrapper.get('[data-removal-kind="batch"]');
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
});

async function mountLibrary() {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [{ path: '/', component: LibraryView }],
    });
    await router.push('/');
    await router.isReady();
    return mount(LibraryView, { attachTo: document.body, global: { plugins: [router] } });
}

function media(id: string, missing: boolean) {
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
        missing,
        categoryIds: [],
    };
}

function snapshotWithLibrary(library: ReturnType<typeof media>[]) {
    return {
        library,
        categories: [],
        settings: defaultSettings(),
        playback: idlePlayback(),
    };
}

function activePlayback(wallpaperId: string) {
    return {
        ...idlePlayback(),
        activeId: wallpaperId,
        status: 'playing' as const,
        displayAssignments: [
            {
                targetId: 'display:primary',
                mode: 'independent' as const,
                displayIds: ['primary'],
                wallpaperId,
                status: 'playing' as const,
                muted: true,
                volume: 0,
                pauseReasons: [],
            },
        ],
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

function display(id: string, x: number, primary: boolean, connected = true) {
    return {
        id,
        name: id,
        x,
        y: 0,
        width: 1920,
        height: 1080,
        primary,
        connected,
    };
}

function displayAssignment(targetId: string, wallpaperId: string) {
    return {
        targetId,
        mode: 'independent' as const,
        displayIds: [targetId],
        wallpaperId,
        status: 'playing' as const,
        muted: true,
        volume: 0,
        pauseReasons: [],
    };
}
