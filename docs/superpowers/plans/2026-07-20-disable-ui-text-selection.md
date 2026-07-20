# Disable UI Text Selection Implementation Plan

<!-- 说明禁止 Wall 非输入界面文字拖选的最小测试先行实施步骤。 -->

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 禁止应用非输入界面的文字拖选，同时保留输入框内的文字选择。

**Architecture:** 在 `.app-window` 这一共享根容器统一禁用选择，并在现有 `input` 元素上恢复文字选择。
使用已有静态样式契约测试锁定两条规则，不添加组件级逻辑。

**Tech Stack:** Vue 3、CSS、Vitest

## Global Constraints

- 仅修复文字选择；不修改右键菜单、视频预览或 Figma。
- 文件保持 UTF-8、CRLF、4 空格缩进，源码单行不超过 120 字符。
- 不新增依赖。

---

### Task 1: Shared selection contract

**Files:**

- Modify: `src/StyleContract.test.ts`
- Modify: `src/styles.css`

**Interfaces:**

- Consumes: 现有 `.app-window` 根容器和 HTML `input` 元素。
- Produces: 非输入界面不可选、输入框文字可选的共享 CSS 契约。

- [ ] **Step 1: Write the failing test**

在 `src/StyleContract.test.ts` 的 `describe` 中增加：

```ts
it('prevents text selection outside inputs', () => {
    expect(css).toContain(
        '.app-window { width: 100%; height: 100%; background: var(--surface-canvas); user-select: none;',
    );
    expect(css).toContain('input { user-select: text;');
});
```

- [ ] **Step 2: Run the focused test to verify it fails**

Run: `npm test -- src/StyleContract.test.ts`

Expected: FAIL because `.app-window` does not disable selection and `input` does not restore it.

- [ ] **Step 3: Write the minimal implementation**

将禁用规则从 `.titlebar` 移到共享根容器，并恢复输入框文字选择：

```css
.app-window {
    width: 100%;
    height: 100%;
    background: var(--surface-canvas);
    user-select: none;
}
input {
    user-select: text;
}
```

- [ ] **Step 4: Run verification**

Run: `npm test -- src/StyleContract.test.ts`

Expected: PASS.

Run: `npm test`

Expected: all tests PASS.

Run: `npm run build`

Expected: TypeScript checking and Vite build both succeed.

- [ ] **Step 5: Commit only this fix**

Stage only the two selection-rule hunks, preserving unrelated working-tree changes, then commit:

```powershell
git commit -m "fix(ui): 禁止非输入界面文字选择"
```
