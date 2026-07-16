<!-- Wall 性能设置页重画实施计划。 -->

# Wall Performance Page Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 删除旧性能内容区，并按常规页、播放页的单层颜色结构重画三个自动暂停设置。

**Architecture:** 保留 `Settings / Performance` 外壳和页签，只替换 `Performance Settings` 子树。新内容区使用一个绑定 `color/surface/panel` 的自动布局卡片；标题、说明、三个透明设置行和两条分割线都位于该卡片内，不再使用第二层表面颜色。

**Tech Stack:** Figma Plugin API、现有 Wall Figma 变量与组件、PowerShell 本地验收。

## Global Constraints

- Figma 文件键固定为 `SdCRz7VSCFwVC3Apk9dniq`，Screens 页面固定为 `5:8`。
- 性能页顶层画板固定为 `44:348`，尺寸保持 1280 × 800 px。
- 保留标题栏、侧栏、页面标题、四个页签及“性能”选中态。
- 保留三个功能和当前开关状态，不新增功能、组件、颜色变量或图标。
- 页面画布使用 `color/surface/canvas`，唯一内容表面使用 `color/surface/panel`。
- 三个设置行必须透明，不得使用 `color/surface/sidebar`、`color/surface/elevated` 或其他第二层底色。
- 文案、字体和 Toggle 组件复用现有节点；其他八张产品界面不修改。

---

### Task 1: 删除并重画性能内容区

**Files:**
- Modify: Figma 文件 `SdCRz7VSCFwVC3Apk9dniq`，节点 `44:348`
- Reference: `docs/superpowers/specs/2026-07-16-performance-page-redesign-design.md`
- Create: `.codex/qa/settings-performance-redesigned.png`
- Modify: `docs/superpowers/plans/2026-07-16-performance-page-redesign.md`

**Interfaces:**
- Consumes: `Settings / Performance`（`44:348`）、旧内容区 `Performance Settings`（`44:420`）、本地颜色变量、旧页面文字节点和三个 Toggle 实例。
- Produces: 一个新的 `Performance Settings` 内容区，以及可复核的新内容区节点 ID 和 QA 截图。

- [x] **Step 1: 读取基线并验证旧结构不符合单层颜色约束**

先读取 `44:348` 的 design context、metadata 和 screenshot。随后执行只读检查：

```js
figma.skipInvisibleInstanceChildren = true;
const page = await figma.getNodeByIdAsync("5:8");
if (!page || page.type !== "PAGE") throw new Error("Screens page not found");
await figma.setCurrentPageAsync(page);
const content = await figma.getNodeByIdAsync("44:420");
if (!content || content.type !== "FRAME") throw new Error("Old performance content not found");
const filledDescendants = content.findAll((node) => {
    if (node.type !== "FRAME") return false;
    if (!("fills" in node) || node.fills === figma.mixed || node.fills.length === 0) return false;
    const fill = node.fills[0];
    return fill.type === "SOLID" && node.id !== content.id;
});
return {
    oldContentId: content.id,
    nestedFilledNodeIds: filledDescendants.map((node) => node.id),
    singleSurface: filledDescendants.length === 0
};
```

Expected: `oldContentId` 为 `44:420`，`singleSurface` 为 `false`，证明当前内容区存在第二层底色。

- [x] **Step 2: 分步搭建新内容区并在完成后替换旧内容区**

为遵守单次写入不超过 10 个逻辑操作的限制，先创建新内容骨架，再逐行填充；新树完整后删除 `44:420`。
所有调用都传入 `skillNames: "figma-use,figma-generate-design"`。下列代码描述最终写入结果，执行时按骨架、三行内容、
最终替换拆分调用：

```js
const page = await figma.getNodeByIdAsync("5:8");
if (!page || page.type !== "PAGE") throw new Error("Screens page not found");
await figma.setCurrentPageAsync(page);

const [oldContent, main, oldHeading, sourceDescription] = await Promise.all([
    figma.getNodeByIdAsync("44:420"),
    figma.getNodeByIdAsync("44:363"),
    figma.getNodeByIdAsync("44:421"),
    figma.getNodeByIdAsync("44:426")
]);
if (!oldContent || oldContent.type !== "FRAME") throw new Error("Old content missing");
if (!main || main.type !== "FRAME") throw new Error("Performance main missing");
if (!oldHeading || oldHeading.type !== "TEXT") throw new Error("Heading source missing");
if (!sourceDescription || sourceDescription.type !== "TEXT") throw new Error("Description source missing");

const titleIds = ["44:424", "44:429", "44:434"];
const descriptionIds = ["44:426", "44:431", "44:436"];
const toggleIds = ["44:425", "44:430", "44:435"];
const [titles, descriptions, toggles, colorVariables] = await Promise.all([
    Promise.all(titleIds.map((id) => figma.getNodeByIdAsync(id))),
    Promise.all(descriptionIds.map((id) => figma.getNodeByIdAsync(id))),
    Promise.all(toggleIds.map((id) => figma.getNodeByIdAsync(id))),
    figma.variables.getLocalVariablesAsync("COLOR")
]);
if (titles.some((node) => !node || node.type !== "TEXT")) throw new Error("Title source missing");
if (descriptions.some((node) => !node || node.type !== "TEXT")) throw new Error("Description source missing");
if (toggles.some((node) => !node || node.type !== "INSTANCE")) throw new Error("Toggle source missing");

const textSources = [oldHeading, sourceDescription, ...titles, ...descriptions];
const fonts = new Map();
for (const text of textSources) {
    for (const segment of text.getStyledTextSegments(["fontName"])) {
        fonts.set(JSON.stringify(segment.fontName), segment.fontName);
    }
}
await Promise.all([...fonts.values()].map((font) => figma.loadFontAsync(font)));

const panelVariable = colorVariables.find((variable) => variable.name === "color/surface/panel");
const borderVariable = colorVariables.find((variable) => variable.name === "color/border/default");
if (!panelVariable || !borderVariable) throw new Error("Required color variables missing");

const createdNodeIds = [];
const content = figma.createAutoLayout("VERTICAL");
content.name = "Performance Settings";
content.resize(832, 296);
content.layoutSizingHorizontal = "FIXED";
content.layoutSizingVertical = "HUG";
content.paddingTop = 14;
content.paddingRight = 16;
content.paddingBottom = 14;
content.paddingLeft = 16;
content.itemSpacing = 12;
content.cornerRadius = oldContent.cornerRadius;
content.clipsContent = true;
content.fills = [figma.variables.setBoundVariableForPaint(
    { type: "SOLID", color: { r: 0, g: 0, b: 0 } },
    "color",
    panelVariable
)];
createdNodeIds.push(content.id);

const header = figma.createAutoLayout("VERTICAL");
header.name = "Performance Header";
header.fills = [];
header.itemSpacing = 6;
content.appendChild(header);
header.layoutSizingHorizontal = "FILL";
header.layoutSizingVertical = "HUG";
createdNodeIds.push(header.id);

const heading = oldHeading.clone();
header.appendChild(heading);
heading.layoutSizingHorizontal = "HUG";
createdNodeIds.push(heading.id);

const subtitle = sourceDescription.clone();
subtitle.characters = "在以下状态下自动暂停壁纸";
header.appendChild(subtitle);
subtitle.layoutSizingHorizontal = "HUG";
createdNodeIds.push(subtitle.id);

const list = figma.createAutoLayout("VERTICAL");
list.name = "Pause Rules";
list.fills = [];
list.itemSpacing = 0;
content.appendChild(list);
list.layoutSizingHorizontal = "FILL";
list.layoutSizingVertical = "HUG";
createdNodeIds.push(list.id);

for (let index = 0; index < 3; index += 1) {
    const row = figma.createAutoLayout("HORIZONTAL");
    row.name = `Pause Rule / ${index + 1}`;
    row.resize(800, 72);
    row.primaryAxisSizingMode = "FIXED";
    row.counterAxisSizingMode = "FIXED";
    row.primaryAxisAlignItems = "SPACE_BETWEEN";
    row.counterAxisAlignItems = "CENTER";
    row.paddingLeft = 16;
    row.paddingRight = 16;
    row.fills = [];
    list.appendChild(row);
    row.layoutSizingHorizontal = "FILL";
    row.layoutSizingVertical = "FIXED";
    createdNodeIds.push(row.id);

    const copy = figma.createAutoLayout("VERTICAL");
    copy.name = "Copy";
    copy.fills = [];
    copy.itemSpacing = 4;
    row.appendChild(copy);
    copy.layoutSizingHorizontal = "HUG";
    copy.layoutSizingVertical = "HUG";
    createdNodeIds.push(copy.id);

    const title = titles[index].clone();
    const description = descriptions[index].clone();
    copy.appendChild(title);
    copy.appendChild(description);
    title.layoutSizingHorizontal = "HUG";
    description.layoutSizingHorizontal = "HUG";
    createdNodeIds.push(title.id, description.id);

    const toggle = toggles[index].clone();
    row.appendChild(toggle);
    createdNodeIds.push(toggle.id);

    if (index < 2) {
        const divider = figma.createRectangle();
        divider.name = "Divider";
        divider.resize(800, 1);
        divider.fills = [figma.variables.setBoundVariableForPaint(
            { type: "SOLID", color: { r: 0, g: 0, b: 0 } },
            "color",
            borderVariable
        )];
        list.appendChild(divider);
        divider.layoutSizingHorizontal = "FILL";
        divider.layoutSizingVertical = "FIXED";
        createdNodeIds.push(divider.id);
    }
}

const oldIndex = main.children.indexOf(oldContent);
if (oldIndex < 0) throw new Error("Old content is not a child of Performance Main");
main.insertChild(oldIndex, content);
content.layoutSizingHorizontal = "FIXED";
content.layoutSizingVertical = "HUG";
const deletedNodeIds = [oldContent.id];
oldContent.remove();
return {
    createdNodeIds,
    mutatedNodeIds: [main.id],
    deletedNodeIds,
    contentId: content.id,
    size: [content.width, content.height]
};
```

Expected: 返回新的 `contentId`；`deletedNodeIds` 只包含 `44:420`；新内容区宽 832 px，高度由自动布局贴合内容。

- [x] **Step 3: 运行结构验收**

对新 `contentId` 调用 `get_metadata`，然后执行只读断言：

```js
figma.skipInvisibleInstanceChildren = true;
const page = await figma.getNodeByIdAsync("5:8");
if (!page || page.type !== "PAGE") throw new Error("Screens page not found");
await figma.setCurrentPageAsync(page);
const screen = await figma.getNodeByIdAsync("44:348");
if (!screen || screen.type !== "FRAME") throw new Error("Performance screen missing");
const contents = screen.findAll((node) => node.name === "Performance Settings");
if (contents.length !== 1 || contents[0].type !== "FRAME") {
    throw new Error(`Expected one performance content frame, got ${contents.length}`);
}
const content = contents[0];
const rows = content.findAll((node) => node.name.startsWith("Pause Rule /"));
const dividers = content.findAll((node) => node.name === "Divider");
const toggles = content.findAllWithCriteria({ types: ["INSTANCE"] })
    .filter((node) => node.name === "Toggle");
const filledRows = rows.filter((node) => node.fills !== figma.mixed && node.fills.length > 0);
const overflows = content.children
    .filter((node) => node.y + node.height > content.height)
    .map((node) => node.id);
return {
    contentId: content.id,
    rows: rows.length,
    dividers: dividers.length,
    toggles: toggles.length,
    filledRows: filledRows.length,
    overflows,
    oldContentExists: Boolean(await figma.getNodeByIdAsync("44:420"))
};
```

Expected: `rows: 3`、`dividers: 2`、`toggles: 3`、`filledRows: 0`、`overflows: []`、`oldContentExists: false`。

- [x] **Step 4: 运行视觉验收并保存截图**

对 `44:348` 和新的 `contentId` 分别调用 `get_screenshot`，最大尺寸 1280 px。将完整页截图保存为：

```text
D:\projects\github\wall\.codex\qa\settings-performance-redesigned.png
```

Expected: 页面只有一个面板底色；三个设置行透明且由两条细分割线区分；标题、说明和开关无裁切、重叠或错位；侧栏、页签与其他设置页一致。

- [x] **Step 5: 更新计划状态并验证文件格式**

将本计划五个步骤勾选为完成，然后执行：

```powershell
$file = 'docs\superpowers\plans\2026-07-16-performance-page-redesign.md'
$bytes = [IO.File]::ReadAllBytes((Resolve-Path $file))
if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
    throw 'Plan contains UTF-8 BOM'
}
$text = [Text.Encoding]::UTF8.GetString($bytes)
if ([Regex]::IsMatch($text, '(?<!\r)\n')) {
    throw 'Plan contains LF-only line endings'
}
if ($text -match '- \[ \]') {
    throw 'Plan contains unchecked steps'
}
```

Expected: 命令退出码 0。当前目录不是 Git 仓库，因此不执行提交。
