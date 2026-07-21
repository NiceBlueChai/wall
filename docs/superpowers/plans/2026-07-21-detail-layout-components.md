# Detail Layout And Components Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate nested detail-card scrollbars and align all playback-setting selects with Figma `03 Components`.

**Architecture:** Keep the existing detail actions and store/API flow. Repair the detail card with one scrollable information
region and a fixed action row so long paths remain fully readable, then replace native selects with one small accessible Vue
component styled from Figma Select `13:42` and Menu Item `15:78`.

**Tech Stack:** Vue 3, TypeScript, CSS, Vitest, Vue Test Utils.

## Global Constraints

- Do not modify Figma; current component definitions are the source of truth.
- Preserve image/video behavior and desktop playback actions.
- Use existing design tokens and SVG icons; add no dependency.
- Use UTF-8 without BOM, CRLF, four-space indentation, and 120-character source lines.

---

### Task 1: Add the shared Figma select

**Files:**

- Create: `src/components/WallSelect.vue`
- Create: `src/components/WallSelect.test.ts`
- Create: `src/playbackOptions.ts`
- Modify: `src/styles.css`

**Interfaces:**

- Consumes: `WallIcon`, existing color tokens, and `{ value, label }` option arrays.
- Produces: `WallSelect` with `modelValue`, `options`, `disabled`, `label`, `update:modelValue`, and `change`.

- [x] **Step 1: Write the failing component test**

Render `WallSelect`, assert the closed combobox label, open it, choose an option, verify the emitted value, then verify the
disabled state cannot open.

- [x] **Step 2: Run the test to verify it fails**

Run: `npm test -- src/components/WallSelect.test.ts`

Expected: FAIL because `WallSelect.vue` does not exist.

- [x] **Step 3: Implement the minimum component and Figma styles**

Use a 36px trigger, 8px radius, panel background, default border, 2px accent open border, 13px label, 16px chevron,
and 36px Menu Item rows. Support mouse selection, Escape, arrows, Home/End, focus return, and disabled state.

- [x] **Step 4: Run the component test**

Run: `npm test -- src/components/WallSelect.test.ts`

Expected: PASS.

### Task 2: Repair detail scroll ownership and migrate callers

**Files:**

- Modify: `src/views/DetailView.vue`
- Modify: `src/views/SettingsView.vue`
- Modify: `src/DetailView.test.ts`
- Modify: `src/SettingsView.test.ts`
- Modify: `src/StyleContract.test.ts`
- Modify: `src/styles.css`

**Interfaces:**

- Consumes: `WallSelect` and the shared playback option arrays from Task 1.
- Produces: a fixed 384x324 detail card with no nested scrolling and Figma-aligned selects on detail/settings pages.

- [x] **Step 1: Write failing integration and style-contract tests**

Assert detail/settings render comboboxes instead of native selects; assert the detail card has exactly one information scroll
owner, long paths wrap without truncation, and the action row remains the final fixed card row.

- [x] **Step 2: Run the focused tests to verify they fail**

Run: `npm test -- src/DetailView.test.ts src/SettingsView.test.ts src/StyleContract.test.ts`

Expected: FAIL on native selects and `overflow: auto` detail rules.

- [x] **Step 3: Implement the minimum caller and CSS changes**

Replace all playback/general native selects, group detail metadata under one scroll owner, remove per-field scrolling, keep
the action row at the bottom, and allow long paths and target labels to wrap in full.

- [x] **Step 4: Run focused and full verification**

Run: `npm test -- src/components/WallSelect.test.ts src/DetailView.test.ts src/SettingsView.test.ts src/StyleContract.test.ts`

Run: `npm test`

Run: `npm run build`

Expected: all commands PASS.

- [x] **Step 5: Capture visual evidence**

At 1280x800, capture image and video detail states. Confirm one page scroll owner, no overlap, fixed bottom actions, and
select/menu parity with Figma.
