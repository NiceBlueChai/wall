/** 验证四个设置页的职责分离，防止版本信息重新泄漏到常规页。 */
// @vitest-environment jsdom

import { flushPromises, mount } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { wallStore, defaultSettings } from './store';
import SettingsView from './views/SettingsView.vue';

vi.mock('./api', () => ({
    updateSettings: vi.fn(),
    openLicense: vi.fn(),
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
        await router.push('/settings/about');
        await flushPromises();
        expect(wrapper.text()).toContain('v1.0.0');
        expect(wrapper.text()).toContain('完全离线');
    });
});
