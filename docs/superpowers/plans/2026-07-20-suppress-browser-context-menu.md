# Suppress Browser Context Menu Implementation Plan

<!-- 说明禁止 Wall 非编辑界面浏览器右键菜单的测试先行实施步骤。 -->

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 禁止普通应用界面的 Chromium 右键菜单，同时保留输入控件的编辑菜单。

**Architecture:** 在共享 `App.vue` 根容器处理冒泡的 `contextmenu` 事件；编辑目标直接放行，其他目标阻止默认行为。
使用现有 App 组件测试同时验证禁用面和输入控件反例。

**Tech Stack:** Vue 3、TypeScript、Vitest、Vue Test Utils

## Global Constraints

- 不创建 Wall 自定义右键菜单，不修改 Figma、视频预览或现有业务操作。
- 文件保持 UTF-8、CRLF、4 空格缩进，源码单行不超过 120 字符。
- 不新增依赖。

---

### Task 1: Shared context-menu boundary

**Files:**

- Modify: `src/App.test.ts`
- Modify: `src/App.vue`

**Interfaces:**

- Consumes: 冒泡到 `.app-window` 的浏览器 `MouseEvent`。
- Produces: `preventBrowserContextMenu(event: MouseEvent): void` 和根容器右键事件绑定。

- [ ] **Step 1: Write the failing test**

在 `src/App.test.ts` 的 `describe` 中增加：

```ts
it('suppresses the browser context menu outside editable controls', async () => {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/', name: 'library', component: { template: '<div />' } },
            { path: '/settings/:section', name: 'settings', component: { template: '<div />' } },
        ],
    });
    await router.push('/');
    await router.isReady();
    const wrapper = mount(App, { global: { plugins: [router] } });

    const shellEvent = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
    wrapper.get('.sidebar').element.dispatchEvent(shellEvent);
    expect(shellEvent.defaultPrevented).toBe(true);

    await wrapper.get('[aria-label="添加分类"]').trigger('click');
    const inputEvent = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
    wrapper.get('.category-dialog input').element.dispatchEvent(inputEvent);
    expect(inputEvent.defaultPrevented).toBe(false);
});
```

- [ ] **Step 2: Run the focused test to verify it fails**

Run: `npm test -- src/App.test.ts`

Expected: FAIL because the shell event is not prevented.

- [ ] **Step 3: Write the minimal implementation**

在 `src/App.vue` 脚本中增加：

```ts
function preventBrowserContextMenu(event: MouseEvent) {
    const target = event.target;
    if (target instanceof Element && target.closest('input, textarea, [contenteditable="true"]')) return;
    event.preventDefault();
}
```

将根容器改为：

```vue
<div class="app-window" @contextmenu="preventBrowserContextMenu"></div>
```

- [ ] **Step 4: Run verification**

Run: `npm test -- src/App.test.ts`

Expected: PASS.

Run: `npm test`

Expected: all tests PASS.

Run: `npm run build`

Expected: TypeScript checking and Vite build both succeed.

- [ ] **Step 5: Commit only this fix**

Stage only `src/App.test.ts` and the two context-menu hunks in `src/App.vue`, preserving unrelated working-tree changes, then commit:

```powershell
git commit -m "fix(ui): 禁止非编辑界面浏览器右键菜单"
```
