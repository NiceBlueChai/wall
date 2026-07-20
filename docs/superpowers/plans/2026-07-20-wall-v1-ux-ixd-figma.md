<!-- Wall v1 UX / IxD 文档与 Figma 交付的实施计划。 -->

# Wall v1 UX / IxD Figma Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在现有 Figma 文件中补齐 Wall v1 用户旅程、状态模型、交互契约、恢复路径和正式画板索引。

**Architecture:** `05 Screens` 只承载 1280 × 800 正式页面和分组总览；新建的 `06 UX & Flows` 只承载行为图、
状态与交付注释，不复制完整页面。所有 UI 修正直接作用于现有正式画板，历史重复画板保留为参考。

**Tech Stack:** Figma Design、Figma Plugin API、现有 Wall 变量、Noto Sans SC、现有组件与图标、Markdown。

## Global Constraints

- 只覆盖 Windows 10/11 x64 的当前 v1 离线能力。
- 不新增账号、云同步、在线商店、播放列表、场景编辑器、组件库、变量集合或图标。
- Figma 文件固定为 `SdCRz7VSCFwVC3Apk9dniq`。
- 所有完整产品画板保持 1280 × 800。
- 复用现有颜色变量、文本样式、组件和 `04 Icons`。
- 图片不得出现视频专属播放、暂停、静音、音量、帧率或硬件解码控制。
- 单项、批量、失效清理只移除库记录，不删除或修改源文件。
- 不修改 Vue、Rust 或 Tauri 产品代码。

---

### Task 1: 建立正式画板清单与设计系统映射

**Files:**

- Read: `docs/design/wall-v1-design-spec.md`
- Read: `docs/design/wall-v1-ux-ixd-spec.md`
- Read: Figma `05 Screens` (`5:8`)
- Read: Figma `03 Components`、`04 Icons` 和本地变量集合

**Interfaces:**

- Consumes: 现有 38 张顶层画板、23 张旧总览、5 张库管理状态和 2 张批量分类菜单。
- Produces: 30 张正式状态、历史参考状态、可复用组件、变量与文本样式的准确映射。

- [x] **Step 1: 读取 `05 Screens` 顶层节点**

使用一次只读 `use_figma` 调用切换到 `5:8`，返回所有顶层节点的 ID、名称、位置、尺寸和类型。

- [x] **Step 2: 分类正式与历史状态**

正式状态由现有 23 张总览页面、`132:3216`、`132:3288`、`144:3271`、`144:3630`、`144:3780`、
`144:3920`、`144:4079` 组成，共 30 张。其余重复状态进入历史参考区，不删除。

- [x] **Step 3: 读取设计系统来源**

检查正式壁纸库和详情画板使用的组件实例、变量绑定、文本样式与字体。只有文件内不存在对应资产时才使用普通
Frame 和 Text；不创建新组件、变量或图标。

- [x] **Step 4: 验证映射**

返回正式节点数量 `30`、全部尺寸 `1280 × 800`，并确认 Noto Sans SC 的实际可用字重。

### Task 2: 创建 `06 UX & Flows` 页面骨架

**Files:**

- Modify: Figma 文件 `SdCRz7VSCFwVC3Apk9dniq`

**Interfaces:**

- Consumes: Task 1 的设计系统映射和 `wall-v1-ux-ixd-spec.md`。
- Produces: 一个页面和六个顶层区块，后续任务直接使用返回的稳定节点 ID。

- [x] **Step 1: 创建页面与总标题**

创建唯一页面 `06 UX & Flows`。若同名页面已存在则复用，不创建第二个。页面顶部建立标题、版本、范围和 Figma
文件说明。

- [x] **Step 2: 创建六个区块容器**

依次创建 `01 Scope & Principles`、`02 Journey Map`、`03 State Models`、`04 Interaction Contract`、
`05 Recovery Matrix`、`06 Screen Index`。容器使用纵向 Auto Layout，区块间距 160 px，并返回全部节点 ID。

- [x] **Step 3: 截图验证骨架**

逐区块截图，确认没有重叠、裁切或遗留 placeholder 状态。

### Task 3: 完成范围、旅程与状态模型

**Files:**

- Modify: Figma `06 UX & Flows`

**Interfaces:**

- Consumes: Task 2 的前三个区块 ID。
- Produces: 五条端到端旅程、播放状态模型、布局草稿模型和异步操作模型。

- [x] **Step 1: 填充范围与原则**

写入本地优先、状态明确、草稿后提交、安全移除、系统边界和非目标。每张原则卡只表达一个判断。

- [x] **Step 2: 绘制五条主旅程**

分别绘制首次导入与播放、浏览预览、多显示器、库管理、后台托盘。每条使用“用户动作 → 系统反馈 → 稳定结果”
三层，并增加一条取消路径和一条失败恢复路径。

- [x] **Step 3: 绘制三套状态模型**

播放模型覆盖空闲、运行、暂停、错误；显示布局覆盖已提交、编辑草稿、无效草稿、提交中、失败；异步操作覆盖
就绪、提交中、成功和可恢复错误。

- [x] **Step 4: 截图验证**

分别截图旅程和状态区，确认连线方向一致、中文完整、错误与取消路径可辨认。

### Task 4: 完成交互契约与恢复矩阵

**Files:**

- Modify: Figma `06 UX & Flows`

**Interfaces:**

- Consumes: Task 2 的交互与恢复区块 ID。
- Produces: 菜单、弹层、弹窗、键盘、焦点、媒体差异及异常恢复规范。

- [x] **Step 1: 制作层级与关闭规则表**

覆盖下拉菜单、显示器弹层、普通弹窗、危险确认和导入中状态；列出 Esc、外部点击、初始焦点和焦点返回。

- [x] **Step 2: 制作输入规则表**

覆盖 Tab、Enter、Space、方向键、Home、End 和 Esc；明确批量模式与普通模式下卡片行为。

- [x] **Step 3: 制作视频与图片差异表**

视频显示应用内预览、暂停、声音、帧率和硬件解码；图片只保留静态预览、缩放、画幅和抗锯齿。

- [x] **Step 4: 制作恢复矩阵**

覆盖空库、分类空、搜索无结果、文件丢失、播放失败、屏幕离线、设置保存失败和项目已移除。

- [x] **Step 5: 截图验证**

以足够分辨率检查表格标题、长文案、危险说明和焦点规则无裁切。

### Task 5: 修正正式 UI 画板

**Files:**

- Modify: Figma `05 Screens` (`5:8`)

**Interfaces:**

- Consumes: UX/IxD 契约和现有正式画板。
- Produces: 与行为契约一致的壁纸库、详情、管理和状态摘要。

- [x] **Step 1: 修正壁纸库卡片操作**

在标准库画板 `93:422` 增加低干扰的卡片快捷播放入口说明，移除双击播放提示；批量画板 `95:506` 保持卡片只选择。

- [x] **Step 2: 修正详情页媒体语义**

在视频详情 `105:1395` 明确“视频预览”控制区与“桌面壁纸”控制区；在图片详情 `105:1531` 确认没有继续、
暂停、静音、音量、帧率或硬件解码控件。

- [x] **Step 3: 修正多目标状态摘要**

正式画板中的侧栏底部状态使用目标数量和“运行中 / 部分暂停 / 全部暂停 / 空闲”摘要，不再假设只有一张当前壁纸。

- [x] **Step 4: 修正危险操作层级**

批量画板 `144:3780` 使用紧凑、低强调的“移出库”入口；`144:3630`、`144:3920`、`144:4079` 的实心危险色
只用于确认按钮，并保留数量、运行影响和源文件说明。

- [x] **Step 5: 验证关键画板**

截图 `93:422`、`95:506`、`105:1395`、`105:1531`、`144:3630`、`144:3780`、`144:3920`、`144:4079`，
逐张确认 1280 × 800、无重叠、无裁切、媒体语义正确。

### Task 6: 更新总览、索引和原型连接

**Files:**

- Modify: Figma `05 Screens` (`5:8`)
- Modify: Figma `06 UX & Flows`

**Interfaces:**

- Consumes: 30 张正式状态和前述所有 UX 区块。
- Produces: 可查找的总览、屏幕索引和最小关键路径原型。

- [x] **Step 1: 更新总览**

把 `115:1747` 从 23 张扩展到 30 张，按壁纸库、显示器、详情、设置和管理确认分组；标题改为
“冻结候选总览 · 30 个完整状态 · 缩略比例 25%”。

- [x] **Step 2: 标记历史参考**

保留历史画板原坐标，统一添加 `[Legacy]` 前缀，并在画布顶部增加 `Legacy / Reference` 说明。

- [x] **Step 3: 填充 Screen Index**

按分组列出 30 张正式画板的名称、节点 ID、入口和主要退出结果。

- [x] **Step 4: 添加最小原型连接**

从标准库连接导入、详情预览、显示布局、批量移除、失效清理、单项移除和设置切换；取消返回来源状态，成功返回
稳定状态。重复按钮不创建冗余连接。

- [x] **Step 5: 验证导航**

检查所有连接目标存在，起点为标准库，且没有指向历史参考画板的正式流程。

### Task 7: 最终验证与文档同步

**Files:**

- Modify: `docs/design/wall-v1-design-spec.md`
- Modify: `docs/design/wall-v1-ux-ixd-spec.md`
- Modify: `docs/superpowers/plans/2026-07-20-wall-v1-ux-ixd-figma.md`

**Interfaces:**

- Consumes: Figma 最终节点 ID、截图和 30 张正式画板。
- Produces: 可追溯的 GitHub 规范、完成勾选和验证证据。

- [x] **Step 1: 回填 Figma 节点映射**

在两份设计规范中记录 `06 UX & Flows` 页面及六个区块的实际节点 ID，并把旧的“23 张”改为“30 张”。

- [x] **Step 2: 执行文本与尺寸校验**

使用只读 `use_figma` 返回所有正式画板尺寸、空文本、溢出文本、重叠顶层节点和原型目标有效性。

- [x] **Step 3: 执行视觉校验**

生成 `05 Screens` 总览、`06 UX & Flows` 全页及六个区块截图；逐张检查中文、对比度、连线、层级和状态语义。

- [x] **Step 4: 更新计划勾选与检查文档**

运行：

```powershell
git diff --check -- docs/design/wall-v1-design-spec.md docs/design/wall-v1-ux-ixd-spec.md `
    docs/superpowers/plans/2026-07-20-wall-v1-ux-ixd-figma.md
```

预期：退出码 `0`，没有空白错误。

- [x] **Step 5: 提交文档**

```powershell
git add -- docs/design/wall-v1-design-spec.md docs/design/wall-v1-ux-ixd-spec.md `
    docs/superpowers/plans/2026-07-20-wall-v1-ux-ixd-figma.md
git commit -m "docs(design): 完成壁纸体验与交互交付"
```
