# 详情页长标题 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让详情页长文件名保持单行省略，且不挤压或折行右侧操作按钮。

**Architecture:** 继续使用现有详情页头和按钮，只补齐 Flex 收缩边界与原生 `title`。Figma 仅修改现有视频、图片详情节点的标题约束；Vue 只改详情模板和共享样式，不新增组件、状态或依赖。

**Tech Stack:** Figma Plugin API、Vue 3、TypeScript、CSS、Vitest、Tauri 2。

## Global Constraints

- 页头保持 44px 高，标题只显示一行。
- 完整名称保留在页头悬停提示和右侧详情卡中。
- 返回按钮和右侧操作区不可收缩，按钮文字不可换行。
- 不新增 Figma 画板、组件、变量、图标、代码依赖或响应式断点。
- 保留当前未提交的分类编辑和媒体元数据修正；不暂存 `src-tauri/Cargo.toml`。
- 源码使用 UTF-8 无 BOM、CRLF、四空格缩进，单行不超过 120 字符。

---

### Task 1: 更新现有 Figma 页头约束

**Figma nodes:**

- Modify: 视频详情 `105:1395`，页头标题组 `105:1412`，标题 `105:1414`。
- Modify: 图片详情 `105:1531`，页头标题组 `105:1548`，标题 `105:1550`。

**Interfaces:**

- Consumes: 现有 1024 × 44 页头、176px `Detail Actions` 和 Noto Sans SC Bold 24px 标题。
- Produces: 824px 标题组、794px 单行省略标题、24px 标题组与操作区间隔。

- [ ] **Step 1: 读取现有节点和截图**

调用 `get_design_context` 与 `get_screenshot` 获取 `105:1395`、`105:1531`。

Expected: 两张画板均为 1280 × 800；页头内容宽 1024px，操作区宽 176px。

- [ ] **Step 2: 修改两个现有标题组**

使用 `use_figma`，加载每个标题当前字体后执行：

```js
const page = await figma.getNodeByIdAsync('5:8');
await figma.setCurrentPageAsync(page);
const nodes = await Promise.all(
    ['105:1412', '105:1414', '105:1548', '105:1550'].map((id) => figma.getNodeByIdAsync(id)),
);
const [videoGroup, videoTitle, imageGroup, imageTitle] = nodes;
const titles = [videoTitle, imageTitle];
const fonts = titles.flatMap((title) => title.getStyledTextSegments(['fontName']).map((part) => part.fontName));
await Promise.all([...new Map(fonts.map((font) => [JSON.stringify(font), font])).values()].map(figma.loadFontAsync));
for (const group of [videoGroup, imageGroup]) {
    group.resize(824, 29);
    group.layoutSizingHorizontal = 'FIXED';
    group.name = 'Header Identity / Fixed';
}
for (const title of titles) {
    title.textAutoResize = 'NONE';
    title.resize(794, 29);
    title.layoutSizingHorizontal = 'FIXED';
    title.textTruncation = 'ENDING';
    title.maxLines = 1;
    title.name = 'Title / Single-line Truncate';
}
return { mutatedNodeIds: nodes.map((node) => node.id) };
```

Expected: 不改变标题字符、按钮实例、颜色、字体或其他页面内容。

- [ ] **Step 3: 验证 Figma**

重新截图两个详情节点，并读取四个节点的宽高、`textAutoResize`、`textTruncation` 与 `maxLines`。

Expected: 标题组 824 × 29；标题 794 × 29、`NONE`、`ENDING`、`1`；操作区仍为 176 × 38。

### Task 2: 用测试锁定长标题行为

**Files:**

- Modify: `src/DetailView.test.ts`
- Modify: `src/StyleContract.test.ts`

**Interfaces:**

- Consumes: 现有 `DetailView` 渲染测试和 CSS 契约测试。
- Produces: 完整标题悬停文本和页头单行布局的回归证明。

- [ ] **Step 1: 写失败测试**

在详情渲染测试中把媒体名称替换为长文件名，并添加：

```ts
const heading = wrapper.get('.page-heading h1');
expect(heading.text()).toBe(longName);
expect(heading.attributes('title')).toBe(longName);
```

在样式契约测试中添加：

```ts
expect(css).toContain('.detail-page .page-heading > h1 { min-width: 0; flex: 1; margin-right: 0; overflow: hidden;');
expect(css).toContain('.detail-heading-actions { flex: none; display: flex;');
expect(css).toContain('.detail-heading-actions button { flex: none; display: flex;');
expect(css).toContain('white-space: nowrap;');
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `npm test -- src/DetailView.test.ts src/StyleContract.test.ts`

Expected: FAIL，详情页头没有 `title`，CSS 也没有长标题约束。

### Task 3: 最小实现并完整验证

**Files:**

- Modify: `src/views/DetailView.vue`
- Modify: `src/styles.css`
- Modify: `docs/implementation/category-editor-interaction-report.md`

**Interfaces:**

- Consumes: `item.name`、现有 `.page-heading`、`.detail-heading-actions` 与 `.back-button`。
- Produces: 单行省略页头、完整悬停名称、固定操作区和单行按钮。

- [ ] **Step 1: 添加完整名称提示**

把详情页标题改为：

```vue
<h1 :title="item.name">{{ item.name }}</h1>
```

- [ ] **Step 2: 添加最小布局规则**

在详情页头样式附近添加：

```css
.detail-page .page-heading > h1 {
    min-width: 0;
    flex: 1;
    margin-right: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.detail-heading-actions {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
}
.detail-heading-actions button {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
}
.back-button {
    flex: none;
}
```

- [ ] **Step 3: 运行聚焦测试**

Run: `npm test -- src/DetailView.test.ts src/StyleContract.test.ts`

Expected: 两个测试文件全部通过。

- [ ] **Step 4: 更新实现报告**

在分类编辑实现报告的验证结果中补充：长标题保持单行省略，操作区不收缩，详情卡保留完整名称。

- [ ] **Step 5: 运行完整回归**

Run: `npm test`

Expected: 8 个测试文件全部通过。

Run: `npm run build`

Expected: Vue / TypeScript 检查和 Vite 生产构建通过。

- [ ] **Step 6: 桌面视觉验收**

运行 `npm run tauri dev`，在 1280 × 800 窗口打开长名称视频详情。

Expected: 页头一行省略；两个按钮均为单行；详情卡完整名称可换行；预览和设置区域位置不变。

- [ ] **Step 7: 提交实现**

```powershell
git add -- src/views/DetailView.vue src/DetailView.test.ts src/StyleContract.test.ts src/styles.css `
    src/App.vue src/App.test.ts src/LibraryView.test.ts src/views/LibraryView.vue src/categoryCreator.ts `
    docs/implementation/category-editor-interaction-report.md
git diff --cached --check
git commit -m "fix(detail): 完善长标题与分类编辑交互"
```

Expected: `src-tauri/Cargo.toml` 保持未暂存；提交包含已经验证的详情、分类和媒体元数据修正。
