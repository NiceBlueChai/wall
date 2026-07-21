/** 验证 Wall Select 复用 Figma Select 与 Menu Item 的选择、键盘和禁用契约。 */
// @vitest-environment jsdom

import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import WallSelect from './WallSelect.vue';

describe('WallSelect', () => {
    const options = [
        { value: 'original', label: '原始' },
        { value: 'screen', label: '屏幕' },
        { value: 'ratio16x9', label: '16:9' },
    ];

    it('selects an option from the Figma-aligned menu and supports keyboard dismissal', async () => {
        const wrapper = mount(WallSelect, {
            props: {
                modelValue: 'original',
                options,
                label: '画幅',
            },
        });
        const trigger = wrapper.get('[role="combobox"]');

        expect(trigger.text()).toContain('原始');
        expect(trigger.attributes('aria-expanded')).toBe('false');
        await trigger.trigger('click');
        expect(wrapper.findAll('[role="option"]')).toHaveLength(3);
        expect(trigger.attributes('aria-expanded')).toBe('true');

        await wrapper.get('[role="option"][data-value="ratio16x9"]').trigger('click');
        expect(wrapper.emitted('change')).toEqual([['ratio16x9']]);
        expect(wrapper.find('[role="listbox"]').exists()).toBe(false);

        await trigger.trigger('click');
        await trigger.trigger('keydown', { key: 'Escape' });
        expect(wrapper.find('[role="listbox"]').exists()).toBe(false);

        await wrapper.setProps({ disabled: true });
        await trigger.trigger('click');
        expect(trigger.attributes('disabled')).toBeDefined();
        expect(wrapper.find('[role="listbox"]').exists()).toBe(false);
    });
});
