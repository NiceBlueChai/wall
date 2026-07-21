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
        expect(css).toContain('--text-muted: #7d85a3;');
        expect(css).toContain('.wallpaper-card.active { border: 2px solid var(--accent);');
        expect(css).toContain('.segmented button.active { color: var(--text-primary); background: var(--accent);');
        expect(css).toContain('.danger { background: var(--danger);');
        expect(css).toContain('.button-medium { min-height: 40px; padding: 0 16px;');
        expect(css).toContain(
            '.card-quick-play { position: absolute; top: 10px; right: 10px; width: 36px; height: 36px;',
        );
    });

    it('keeps the Figma detail layout inside the 1280 by 800 window', () => {
        expect(css).toContain('.detail-grid { display: grid; grid-template-columns: 620px 384px; gap: 20px; }');
        expect(css).toContain('.media-preview { position: relative; height: 324px;');
        expect(css).toContain('.detail-card { height: 324px;');
        expect(css).toContain('.detail-settings { height: 280px;');
        expect(css).toContain(
            '.detail-settings-primary { display: grid; grid-template-columns: 360px 190px 190px 184px; gap: 20px;',
        );
        expect(css).toContain(
            '.detail-settings-video { display: grid; grid-template-columns: 180px 180px 584px; gap: 20px;',
        );
        expect(css).toContain('.main-content { min-width: 0; min-height: 0; overflow: auto;');
        expect(css).not.toContain('.detail-card p { max-height: 64px; overflow: auto;');
        expect(css).not.toContain('.detail-targets { display: grid; gap: 4px; max-height: 72px; overflow: auto;');
        expect(css).not.toContain('max-height: 58px; overflow: auto; margin-top: 10px;');
        expect(css).toContain('.detail-card-content { min-height: 0; flex: 1; overflow-y: auto;');
        expect(css).toContain('.detail-actions { min-height: 34px; flex: none;');
        expect(css).toContain('.path-copy { display: block; color: var(--text-muted); overflow-wrap: anywhere;');
        expect(css).not.toContain('.path-copy { display: block; overflow: hidden;');
    });

    it('keeps the Figma Select and Menu Item dimensions', () => {
        expect(css).toContain('.wall-select-trigger { width: 100%; height: 36px;');
        expect(css).toContain('.wall-select.open .wall-select-trigger { padding: 0 9px 0 11px; border: 2px solid');
        expect(css).toContain('.wall-select.open .wall-select-trigger:focus-visible { outline: 0; }');
        expect(css).toContain('.wall-select-option { width: 100%; height: 36px;');
    });

    it('prevents text selection outside inputs', () => {
        expect(css).toContain(
            '.app-window { width: 100%; height: 100%; background: var(--surface-canvas); user-select: none;',
        );
        expect(css).toContain('input { user-select: text;');
    });

    it('locks background scrolling while a modal dialog is open', () => {
        expect(css).toContain('html:has(.modal-scrim) .main-content { overflow: hidden; }');
    });
});
