/** 验证自定义标题栏按钮调用原生窗口 API，并拥有对应 Tauri 权限。 */
// @vitest-environment jsdom

import { mount } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import capability from '../src-tauri/capabilities/default.json';
import App from './App.vue';

const { closeMock, maximizeMock, minimizeMock } = vi.hoisted(() => ({
    closeMock: vi.fn(),
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
    listenToSnapshots: vi.fn(async () => () => undefined),
}));

describe('App window controls', () => {
    beforeEach(() => {
        closeMock.mockReset();
        maximizeMock.mockReset();
        minimizeMock.mockReset();
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
});
