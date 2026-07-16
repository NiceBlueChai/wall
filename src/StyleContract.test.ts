/** 静态验证共享样式持续遵循 Figma 基础组件的尺寸与状态约束。 */

import { describe, expect, it } from 'vitest';
// @ts-expect-error Vitest runs in Node; the browser application deliberately omits Node typings.
import { readFileSync } from 'node:fs';

const css = readFileSync(new URL('./styles.css', import.meta.url), 'utf8').replace(/\s+/g, ' ');

describe('Figma style contract', () => {
    it('keeps shared window, card and empty-state geometry', () => {
        expect(css).toContain('.window-controls button { width: 46px;');
        expect(css).toContain('.back-button { width: 32px; height: 32px;');
        expect(css).not.toContain('.back-button::before');
        expect(css).toContain('.card-preview { position: relative; height: 150px;');
        expect(css).toContain('.empty-state { width: 360px; height: 220px;');
        expect(css).toContain('.about-panel { height: 344px; display: flex; flex-direction: column; gap: 16px;');
        expect(css).toContain('.about-contact { display: grid; grid-template-columns: 77px auto; row-gap: 4px;');
    });

    it('keeps Figma action and selection colors', () => {
        expect(css).toContain('.wallpaper-card.active { border: 2px solid var(--accent);');
        expect(css).toContain('.segmented button.active { color: var(--text-primary); background: var(--accent);');
        expect(css).toContain('.danger { background: var(--danger);');
        expect(css).toContain('.button-medium { min-height: 40px; padding: 0 16px;');
    });
});
