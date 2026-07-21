# Category Editor Interaction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make category editing usable from detail and batch surfaces in both populated and zero-category states, while
realigning the detail information card with the approved Figma hierarchy.

**Architecture:** Keep `App.vue` as the single owner of the existing category dialog and expose its create action to routed
views through one typed Vue injection key. Reuse the existing `createCategory` and `setCategoryMembership` calls in sequence,
preserving the approved partial-success message if assignment fails. Update existing Figma frames and components before
translating the approved measurements into Vue/CSS.

**Tech Stack:** Figma MCP, Vue 3, TypeScript, CSS, Vitest, Tauri 2.

## Global Constraints

- Modify existing Figma detail frames and component variants; do not add duplicate top-level screens.
- Preserve the approved 384 × 324 detail card, one scrollable information region, fixed bottom actions, and full paths.
- Category creation, removal, rename, and assignment never delete or modify source media files.
- Reuse existing icons, color variables, dialogs, `Category Tag`, and `Menu Item`; add no dependency.
- Preserve current uncommitted path-prefix and image-note fixes, and exclude `src-tauri/Cargo.toml` from every commit.
- Use UTF-8 without BOM, CRLF, four-space indentation, and Chinese Conventional Commit messages.

---

### Task 1: Preserve the accepted pending detail fixes

**Files:**

- Modify: `src/views/DetailView.vue`
- Modify: `src/DetailView.test.ts`
- Modify: `src/styles.css`

**Interfaces:**

- Consumes: the current working-tree changes that hide `\\?\` only for display and remove the image-only explanation.
- Produces: an isolated checkpoint commit before category work touches the same files.

- [ ] **Step 1: Verify the pending behavior**

Run: `npm test -- src/DetailView.test.ts`

Expected: 8 tests PASS, including the extended-path display assertion and absence of the image explanation.

- [ ] **Step 2: Verify only the intended files are staged**

Run:

```powershell
git add -- src/views/DetailView.vue src/DetailView.test.ts src/styles.css
git diff --cached --check
git status --short
```

Expected: the three files above are staged; `src-tauri/Cargo.toml` remains unstaged.

- [ ] **Step 3: Commit the accepted detail fix**

Run:

```powershell
git commit -m "fix(ui): 优化详情路径与图片说明"
```

Expected: one commit containing only the three staged files.

### Task 2: Update the existing Figma interaction source

**Figma nodes:**

- Modify: video detail `105:1395`
- Modify: image detail `105:1531`
- Modify: `Category Tag` `84:465`
- Modify: `Menu Item` `15:78`
- Modify: category menu `88:120` and batch category menu `132:165`

**Interfaces:**

- Consumes: `docs/superpowers/specs/2026-07-21-category-editor-interaction-design.md`.
- Produces: existing detail and menu nodes with populated and empty category variants for code implementation.

- [ ] **Step 1: Read the exact nodes before editing**

Use Figma MCP `get_design_context` and `get_screenshot` for `105:1395`, `105:1531`, `84:465`, and `132:165`.

Expected: screenshots and structured measurements are available for every changed surface.

- [ ] **Step 2: Update the existing detail frames**

Use `use_figma` after loading the mandatory `figma-use` skill. Keep the information card at 384 × 324 and set:

```text
left/right padding: 20px
title y: 18px
media metadata y: 50px
full path y: 74px
category heading y: 108px
category row y: 132px
category tag height: 28px
category row gap: 8px
```

Replace the standalone “编辑分类” control with existing tags followed by a compact “＋ 添加分类” action. Keep the accepted
scrolling information region and fixed bottom actions when long content needs more height than the static example.

- [ ] **Step 3: Update existing category menu variants**

Keep menu width 208px and Menu Item height 36px. Add these states inside the existing menu/component set:

```text
Populated: existing categories, assigned rows checked and disabled, separator, 新建分类
Empty: 还没有分类, 新建并添加
Batch remove empty: trigger disabled; no empty overlay opens
```

Do not create new icons, variables, or top-level 1280 × 800 frames.

- [ ] **Step 4: Capture Figma proof**

Capture screenshots for video detail populated, image detail empty, category menu empty, and batch add menu empty.

Expected: labels are uncropped, card padding is 20px, menu rows are 36px, and every empty state has a recovery action.

### Task 3: Expose the existing category dialog to routed views

**Files:**

- Create: `src/categoryCreator.ts`
- Modify: `src/App.vue`
- Modify: `src/App.test.ts`

**Interfaces:**

- Consumes: existing `createCategory(name)` and `setCategoryMembership(mediaIds, categoryId, true)` API calls.
- Produces: `openCategoryCreatorKey: InjectionKey<OpenCategoryCreator>` where
  `OpenCategoryCreator = (mediaIds: string[], trigger: HTMLElement) => void`.

- [ ] **Step 1: Write failing provider, success, and partial-success tests**

Extend `App.test.ts` with a probe route component that calls the injected action with `['video-1']`. Make the existing
`createCategoryMock` resolve a snapshot containing one new category and assert:

```text
dialog title: 新建分类
confirm label: 创建并添加
createCategoryMock call: ('动漫收藏')
setCategoryMembershipMock call: (['video-1'], newCategoryId, true)
focus after success: original route trigger
```

Add a second test where creation succeeds and membership fails. Assert the dialog closes, focus returns to the route trigger,
the new category remains in the applied snapshot, and the error toast reads “分类已创建，但添加到当前壁纸失败，请重试”.
Retain the sidebar test and assert it never calls `setCategoryMembership`.

- [ ] **Step 2: Run the test to verify it fails**

Run: `npm test -- src/App.test.ts`

Expected: FAIL because the injection key and create-and-assign workflow do not exist.

- [ ] **Step 3: Add the typed injection contract**

Create `src/categoryCreator.ts` with a file comment and public API comments:

```ts
import type { InjectionKey } from 'vue';

/** 从路由页面打开应用级分类创建弹窗。 */
export type OpenCategoryCreator = (mediaIds: string[], trigger: HTMLElement) => void;

/** 应用级分类创建入口。 */
export const openCategoryCreatorKey: InjectionKey<OpenCategoryCreator> = Symbol('openCategoryCreator');
```

In `App.vue`, `provide` a function that opens the existing create dialog and stores a copied media ID array. Keep the existing
`createCategory(name)` call; after it returns, identify the new category by comparing the returned snapshot with the pre-create
category IDs, then call `setCategoryMembership` when the stored media ID array is non-empty.

Rename `windowError` to `appError` so the existing error toast can also report partial success. If assignment fails after a
successful create, close the dialog, keep the returned snapshot, set the exact partial-success message, and restore focus.
Change only the create-and-assign title/button copy; keep sidebar create, rename, delete, validation, Escape, focus trap, and
busy behavior unchanged. Clear stored media IDs whenever the dialog closes or completes.

- [ ] **Step 4: Run the App tests**

Run: `npm test -- src/App.test.ts`

Expected: all App tests PASS, including sidebar create, routed create-and-assign, partial success, focus trap, Escape, and busy
states.

- [ ] **Step 5: Commit the shared dialog entry**

Run:

```powershell
git add -- src/categoryCreator.ts src/App.vue src/App.test.ts
git commit -m "feat(category): 复用创建并添加流程"
```

### Task 4: Repair and align the detail category editor

**Files:**

- Modify: `src/views/DetailView.vue`
- Modify: `src/DetailView.test.ts`
- Modify: `src/styles.css`

**Interfaces:**

- Consumes: `openCategoryCreatorKey`, existing `setCategoryMembership`, and Figma measurements from Task 2.
- Produces: direct-removal tags, populated/empty add menus, and Figma-aligned detail information hierarchy.

- [ ] **Step 1: Write failing rendered-behavior tests**

Add tests that render categories `[]` and assert the open menu contains “还没有分类” and “新建并添加”, then provide an
`OpenCategoryCreator` mock and assert it receives `['video-1']` and the clicked element. Add a populated test asserting assigned
rows are checked/disabled, unassigned rows call `setCategoryMembership`, and tag removal calls it with `assigned = false`.

- [ ] **Step 2: Run the focused tests to verify they fail**

Run: `npm test -- src/DetailView.test.ts src/StyleContract.test.ts`

Expected: FAIL on missing empty-state action, missing injection call, and old category layout rules.

- [ ] **Step 3: Implement the approved detail interaction**

Inject `openCategoryCreatorKey`. Render metadata/path before the category section, add a “分类” heading, render each assigned
category as the existing tag plus an accessible remove button, and place “＋ 添加分类” last. The menu must contain all existing
categories, a separator, and “新建分类”; with zero categories it must contain “还没有分类” and “新建并添加”.

Update CSS to the exact Task 2 measurements. Keep `.detail-card-content` as the only card information scroll owner and keep
`.detail-actions` outside it. Use the existing Menu Item colors and 36px row height rather than new hard-coded styles.

- [ ] **Step 4: Run detail and style tests**

Run: `npm test -- src/DetailView.test.ts src/StyleContract.test.ts`

Expected: all focused tests PASS for video, image, populated categories, empty categories, long paths, and fixed actions.

- [ ] **Step 5: Commit the detail repair**

Run:

```powershell
git add -- src/views/DetailView.vue src/DetailView.test.ts src/styles.css
git commit -m "fix(detail): 对齐分类编辑交互"
```

### Task 5: Close the batch category empty-state gap

**Files:**

- Modify: `src/views/LibraryView.vue`
- Modify: `src/LibraryView.test.ts`
- Modify: `src/styles.css`

**Interfaces:**

- Consumes: `openCategoryCreatorKey` and current `wallStore.selectedMediaIds`.
- Produces: batch add populated/empty menus and a disabled batch remove trigger when no categories exist.

- [ ] **Step 1: Write failing batch behavior tests**

With selected media and `categories: []`, assert “添加到分类” opens a menu containing “还没有分类” and “新建并添加”, the
provided creator receives a copy of all selected IDs, and “从分类移除” is disabled. Retain existing assertions that closing a
menu does not clear the selection.

- [ ] **Step 2: Run the test to verify it fails**

Run: `npm test -- src/LibraryView.test.ts`

Expected: FAIL because the current add menu is empty and remove remains enabled.

- [ ] **Step 3: Implement batch parity**

Inject `openCategoryCreatorKey`, add the shared empty copy/create action to the add menu, and call the provider with
`[...wallStore.selectedMediaIds]`. Disable the remove trigger when `wallStore.snapshot.categories.length === 0`; do not clear
selection on menu close or dialog cancellation.

- [ ] **Step 4: Run the batch tests**

Run: `npm test -- src/LibraryView.test.ts`

Expected: all LibraryView tests PASS for empty, populated, disabled, menu keyboard, busy, and selection-retention states.

- [ ] **Step 5: Commit batch parity**

Run:

```powershell
git add -- src/views/LibraryView.vue src/LibraryView.test.ts src/styles.css
git commit -m "fix(library): 完善批量分类空状态"
```

### Task 6: Full regression and visual closure

**Files:**

- Verify only; do not modify `src-tauri/Cargo.toml`.

**Interfaces:**

- Consumes: all preceding tasks.
- Produces: fresh automated and visual evidence that the class-level interaction contract is closed.

- [ ] **Step 1: Run full automated verification**

Run:

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

Expected: all Vitest files PASS, Vite production build succeeds, all Rust tests PASS, and diff check reports no errors.

- [ ] **Step 2: Run the desktop app**

Run: `npm run tauri dev`

Expected: the 1280 × 800 Wall window opens and remains responsive.

- [ ] **Step 3: Capture the representative visual matrix**

Capture and inspect:

```text
video detail + existing categories
image detail + zero categories
detail empty category menu
batch add empty category menu
```

Expected: no empty overlay, 20px card alignment, 28px tags, 36px menu rows, full paths, one information scrollbar, fixed
bottom actions, and no clipping or overlap.

- [ ] **Step 4: Verify repository scope**

Run: `git status --short`

Expected: only the pre-existing line-ending-only `src-tauri/Cargo.toml` status remains; no generated or unrelated files are
staged.
