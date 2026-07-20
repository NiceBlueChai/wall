/** 验证四个设置页的职责分离，防止版本信息重新泄漏到常规页。 */
// @vitest-environment jsdom

import { flushPromises, mount } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { wallStore, defaultSettings } from './store';
import SettingsView from './views/SettingsView.vue';

const mocks = vi.hoisted(() => ({
    openLicense: vi.fn(),
    openProjectHomepage: vi.fn(),
    updateSettings: vi.fn(),
}));

vi.mock('./api', () => ({
    updateSettings: mocks.updateSettings,
    openLicense: mocks.openLicense,
    openProjectHomepage: mocks.openProjectHomepage,
}));

describe('SettingsView', () => {
    beforeEach(() => {
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
        expect(wrapper.findAll('.about-contact dt').map((node) => node.text())).toEqual(['作者：', '联系邮箱：']);
        expect(wrapper.findAll('.about-contact dd').map((node) => node.text())).toEqual([
            'NiceBlueChai',
            'bluechai@qq.com',
        ]);
        await wrapper.get('.button-row button').trigger('click');
        await flushPromises();
        expect(mocks.openLicense).toHaveBeenCalledOnce();

        const homepageButton = wrapper.findAll('.button-row button')[1];
        expect(homepageButton.attributes('disabled')).toBeUndefined();
        await homepageButton.trigger('click');
        await flushPromises();
        expect(mocks.openProjectHomepage).toHaveBeenCalledOnce();
    });

    it('exposes the confirmed picture, sound and maximized-pause settings', async () => {
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/settings/:section', component: SettingsView }],
        });
        await router.push('/settings/playback');
        await router.isReady();
        const wrapper = mount(SettingsView, { global: { plugins: [router] } });

        expect(wrapper.text()).toContain('画面');
        expect(wrapper.text()).toContain('声音');
        const scaleModeButtons = wrapper.findAll('.segmented.compact button');
        expect(scaleModeButtons.map((button) => button.text())).toEqual(['填充', '适应', '拉伸']);
        await scaleModeButtons[1].trigger('click');
        await flushPromises();
        expect(mocks.updateSettings).toHaveBeenCalledWith(expect.objectContaining({ scaleMode: 'contain' }));
        expect(wrapper.get('[data-setting="aspect-ratio"]').text()).toContain('21:9');
        expect(wrapper.get('[data-setting="anti-aliasing"]').text()).toContain('高质量');
        expect(wrapper.get('[data-setting="frame-rate"]').text()).toContain('源帧率');

        await wrapper.get('[data-setting="aspect-ratio"]').setValue('ratio21x9');
        await flushPromises();
        expect(mocks.updateSettings).toHaveBeenCalledWith(expect.objectContaining({ aspectRatio: 'ratio21x9' }));

        await router.push('/settings/performance');
        await flushPromises();
        expect(wrapper.text()).toContain('最大化应用时');
        expect(wrapper.findAll('[role="switch"]')).toHaveLength(4);
    });
});
