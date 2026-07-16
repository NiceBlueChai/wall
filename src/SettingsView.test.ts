/** 验证四个设置页的职责分离，防止版本信息重新泄漏到常规页。 */
// @vitest-environment jsdom

import { flushPromises, mount } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { wallStore, defaultSettings } from './store';
import SettingsView from './views/SettingsView.vue';

const mocks = vi.hoisted(() => ({ openLicense: vi.fn(), updateSettings: vi.fn() }));

vi.mock('./api', () => ({
    updateSettings: mocks.updateSettings,
    openLicense: mocks.openLicense,
    openProjectHomepage: vi.fn(),
}));

describe('SettingsView', () => {
    beforeEach(() => {
        wallStore.applySnapshot({
            library: [],
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
    });

    it('shows version only on the about tab', async () => {
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/settings/:section', component: SettingsView }],
        });
        await router.push('/settings/general');
        await router.isReady();
        const wrapper = mount(SettingsView, { global: { plugins: [router] } });

        expect(wrapper.text()).not.toContain('v1.0.0');
        await wrapper.get('.toggle').trigger('click');
        await flushPromises();
        expect(mocks.updateSettings).toHaveBeenCalledWith(expect.objectContaining({ autoStart: true }));

        await router.push('/settings/playback');
        await flushPromises();
        expect(wrapper.get('input[type="range"]').attributes('style')).toContain('--range-progress: 0%');

        await router.push('/settings/about');
        await flushPromises();
        expect(wrapper.text()).toContain('v1.0.0');
        expect(wrapper.text()).toContain('完全离线');
        expect(wrapper.get('.about-brand img').attributes('width')).toBe('48');
        await wrapper.get('.button-row button').trigger('click');
        await flushPromises();
        expect(mocks.openLicense).toHaveBeenCalledOnce();
    });
});
