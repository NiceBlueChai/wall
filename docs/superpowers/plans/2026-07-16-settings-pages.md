<!-- Wall 四页设置界面的 Figma 修补实施计划。 -->

# Wall Four Settings Pages Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** 将 Figma 中的设置设计补齐为常规、播放、性能、关于四张完整界面，并使内容与离线产品规范一致。

**Architecture:** 复用 `Settings / General` 的完整窗口外壳、四页签和现有组件，不创建新的设计系统组件。播放页保留播放控制并移除自动暂停区域；性能页承接三个自动暂停规则；关于页只展示本地版本、许可证和用户主动打开的项目主页。

**Tech Stack:** Figma MCP、Figma Plugin API、现有 Wall Tokens、现有组件实例、Noto Sans SC。

## Global Constraints

- 目标平台为 Windows 10/11 x64，v1 完全离线。
- 所有设置界面尺寸为 1280 × 800 px，标题栏 48 px，侧栏 208 px。
- 四页共用“常规 / 播放 / 性能 / 关于”页签，只替换内容区。
- 默认缩放方式为 Cover；v1 只管理主显示器。
- 不新增变量、组件或依赖；复用现有 Toggle、Select、Slider、Segmented Control、Button 和 App Icon。
- Figma 文件为 `SdCRz7VSCFwVC3Apk9dniq`，目标页面为 `05 Screens`（`5:8`）。
- 当前目录不是 Git 仓库；本计划不初始化仓库，也不执行提交命令。

## Files

- Modify: Figma `05 Screens` page (`5:8`)
- Modify: `docs/design/wall-v1-design-spec.md`
- Modify: `.codex/figma-state-wall-v1.json`

---

### Task 1: 验证现有设置结构并整理画板位置

**Interfaces:**
- Consumes: `Settings / General`（`22:127`）、`Settings / Playback`（`23:155`）。
- Produces: 四页设置共用的外壳结构和无重叠画板坐标。

- [x] **Step 1: 读取两张设置界面的结构和截图**

使用 `get_design_context` 和 `get_screenshot` 检查 `22:127`、`23:155`。确认两张界面都包含 Title Bar、Body、Sidebar、Main 和四个页签。

- [x] **Step 2: 验证现有结构**

运行只读检查：

```js
const ids = ["22:127", "23:155"];
const result = [];
for (const id of ids) {
    const node = await figma.getNodeByIdAsync(id);
    result.push({
        id,
        name: node.name,
        size: [node.width, node.height],
        tabs: node.findAllWithCriteria({ types: ["TEXT"] })
            .filter((text) => ["常规", "播放", "性能", "关于"].includes(text.characters))
            .map((text) => text.characters)
    });
}
return result;
```

Expected: 两张界面均为 1280 × 800，且各有四个页签文本。

- [x] **Step 3: 整理设置画板位置**

将常规页移动到 `(0, 3600)`，播放页移动到 `(1360, 3600)`；为性能页和关于页预留 `(0, 4500)`、`(1360, 4500)`。只修改顶层画板坐标，不改变内部布局。

- [x] **Step 4: 检查顶层画板无重叠**

运行所有 `05 Screens` 顶层 FRAME 的矩形相交检查，Expected: `overlaps = []`。

### Task 2: 修订播放页并创建性能页

**Interfaces:**
- Consumes: 播放内容 `Playback Settings`（`23:244`）、自动暂停内容 `Pause Rules`（`23:285`）、通用页外壳 `22:127`。
- Produces: `Settings / Playback` 和 `Settings / Performance` 两张职责单一的界面。

- [x] **Step 1: 创建性能页外壳**

克隆 `22:127`，命名为 `Settings / Performance`，放置在 `(0, 4500)`。保留 Title Bar、Sidebar、Main 标题和四页签，删除克隆中的 `General Settings` 与版本信息卡。

- [x] **Step 2: 设置性能页选中态**

将“性能”页签使用选中背景和强调色；常规、播放、关于恢复默认文本与透明背景。字体继续使用 Noto Sans SC。

- [x] **Step 3: 将自动暂停规则迁移到性能页**

克隆 `23:285` 到性能页 Main，放置于 `(24, 117)`，命名为 `Performance Settings`，尺寸调整为 `832 × 480`。三个规则行调整为 `800 × 120`，开关右对齐，内容依次为：

```text
全屏应用运行时 / 游戏或全屏视频运行时暂停壁纸
使用电池时 / 未连接电源时减少耗电
显示器休眠时 / 屏幕关闭后立即暂停播放
```

- [x] **Step 4: 收窄播放页职责**

从播放页内容容器中移除 `Pause Rules`。将 `Playback Settings` 调整为 `832 × 570`，五个设置行宽度改为 `800`，右侧控件对齐到面板右边；保留 Cover、60 FPS、硬件解码、默认静音和音量。

- [x] **Step 5: 验证两页截图**

分别截图播放页和性能页。Expected: 两页都有完整窗口外壳和四页签；播放页没有自动暂停设置；性能页只有三个自动暂停规则；无文字裁切或控件越界。

### Task 3: 创建关于页

**Interfaces:**
- Consumes: 通用页外壳 `22:127`、Title Bar 内的 App Icon、Secondary Button 实例 `21:199`。
- Produces: `Settings / About` 完整界面。

- [x] **Step 1: 创建关于页外壳**

克隆 `22:127`，命名为 `Settings / About`，放置在 `(1360, 4500)`。保留共用外壳和四页签，删除克隆中的 `General Settings` 与版本信息卡，并将“关于”页签设为选中态。

- [x] **Step 2: 创建关于信息面板**

在 Main 的 `(24, 117)` 创建 `832 × 480` 的 `About Wall` 面板。复用 Title Bar 内的 App Icon，并添加以下固定文案：

```text
Wall
v1.0.0
Windows 10/11 x64
免费开源 · 完全离线
本地视频与图片动态壁纸工具
```

- [x] **Step 3: 添加两个本地优先操作**

克隆 `21:199` 两次，标签分别设置为“查看开源许可证”和“打开项目主页”。在下方添加说明：“项目主页仅在点击后交给系统默认浏览器，Wall 自身不请求网络。”不添加检查更新、账号或在线内容入口。

- [x] **Step 4: 验证关于页截图**

Expected: 应用图标、版本、平台、离线说明和两个按钮清晰可见；四页签完整；没有联网更新入口；所有内容位于 1280 × 800 画板内。

### Task 4: 全量验收并更新交付状态

**Interfaces:**
- Consumes: 四张设置界面和现有 `05 Screens` 页面。
- Produces: 9 张通过结构与视觉检查的产品界面，以及同步的本地交付状态。

- [x] **Step 1: 运行结构验收**

检查顶层界面数为 9，设置界面数为 4；检查顶层画板重叠、根节点溢出、可见文本宽高小于等于 1、异常坐标和字体家族。

Expected:

```json
{
    "screens": 9,
    "settingsScreens": 4,
    "overlaps": [],
    "overflow": [],
    "collapsedText": [],
    "runaway": [],
    "fontFamilies": ["Noto Sans SC"]
}
```

- [x] **Step 2: 运行视觉验收**

以 1200 px 最大尺寸截图常规、播放、性能、关于四页，逐张检查标题栏、侧栏、选中页签、设置内容、文字渲染和控件对齐。

- [x] **Step 3: 更新状态文件**

将 `.codex/figma-state-wall-v1.json` 的 `phase` 更新为 `qa-complete`，记录性能页和关于页的新节点 ID，并将 `screens` 改为 9、`settingsScreens` 改为 4。

- [x] **Step 4: 更新设计文档交付措辞**

将 `docs/design/wall-v1-design-spec.md` 中“设置页修订完成后的目标”改为已交付措辞，保留四页职责和完全离线约束。

- [x] **Step 5: 验证本地文件格式**

验证两个本地文件均为 UTF-8 无 BOM、CRLF；JSON 可解析；文档中不存在任何占位标记。Expected: 命令退出码 0。

### Task 5: 统一设置页颜色层级并移除重复版本信息

**Interfaces:**
- Consumes: 四张已交付设置界面和现有颜色变量。
- Produces: 与壁纸库、壁纸详情一致的设置页颜色层级。

- [x] **Step 1: 删除常规页版本卡**

删除 `Settings / General` 中的版本卡；版本、平台和离线状态仅保留在 `Settings / About`。

- [x] **Step 2: 让四页内容面板贴合实际内容**

保留 `color/surface/canvas` 画布和 `color/surface/panel` 内容卡，不新增颜色变量。缩短四个内容卡的固定高度，
消除内容下方的大面积面板色空白，并保持内容卡宽度、位置和内部控件不变。

- [x] **Step 3: 验证结构和视觉结果**

检查常规页不存在版本卡，关于页仍显示唯一版本信息；截图核对四页颜色层级、文字、控件和边界。
