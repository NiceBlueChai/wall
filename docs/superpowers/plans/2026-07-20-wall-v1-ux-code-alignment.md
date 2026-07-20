# Wall v1 UX/IxD 代码对齐实施计划

> 依据已确认的 Figma 画板与 `docs/design/wall-v1-ux-ixd-spec.md`，在现有
> `codex/wallpaper-library-management` 分支完整落地，不修改冻结设计规范。

## 目标与边界

- 将壁纸库、显示目标、详情页、设置页和分类管理的现有实现对齐最终 UX/IxD 契约。
- 所有移除操作只删除壁纸库记录，源文件不变；运行中的目标在确认后停止。
- 图片详情不渲染视频专属控件；视频详情保留软件内预览。
- 不新增运行时依赖，不引入通用 UI 框架或超出当前功能的抽象。

## 所有权与验证矩阵

| 契约面 | 所有文件 | 主要验证 |
| --- | --- | --- |
| 壁纸库与批量管理 | `src/views/LibraryView.vue`、`src/LibraryView.test.ts` | 快捷播放、批量成功态、菜单、显示草稿 |
| 全局状态与分类 | `src/App.vue`、`src/App.test.ts` | 多目标摘要、分类菜单与弹窗焦点 |
| 详情与设置 | `src/views/DetailView.vue`、对应测试 | 媒体类型控件、异步防重、错误恢复 |
| 数据与命令契约 | `src/store.ts`、`src/api.ts`、Rust command/core | 原子移除、扫描、播放目标停止 |
| 视觉规范 | `src/styles.css`、`src/StyleContract.test.ts` | muted token、36×36 快捷按钮、危险操作层级 |
| 交付证据 | `docs/implementation/*.md`、`docs/images/*` | Figma 节点、测试输出、逐页截图 |

当前工作树已有同一功能跨 Vue、Rust 和文档的未提交改动。为避免独立工作树遗漏这些改动或造成共享文件冲突，
本轮在当前分支单线执行，不并行委派。

## 任务 1：卡片快捷播放与稳定批量状态

1. 在 `src/LibraryView.test.ts` 先写失败测试：单击卡片进入详情，双击不播放；专用快捷播放按钮播放且不跳转；
   批量模式和失效项不显示该按钮；批量移除成功退出批量模式，失败保留选择。
2. 在 `src/views/LibraryView.vue` 用卡片内 36×36 播放按钮替代双击契约，复用现有播放命令。
3. 在 `src/styles.css` 对齐 Figma 的悬停、聚焦、边框、圆角和尺寸，并把 muted token 调整为 `#7D85A3`。
4. 运行定向测试，确认先红后绿。

## 任务 2：显示目标草稿、聚合状态和失败恢复

1. 在 `src/LibraryView.test.ts` 写失败测试：离线或重复显示器不可应用；请求失败时面板与草稿保留；重试成功才关闭。
2. 在 `src/App.test.ts` 写多目标运行中、部分暂停、全部暂停和错误摘要测试。
3. 在 `src/views/LibraryView.vue` 增加应用中状态，校验目标数量、唯一性和在线状态；失败不关闭面板。
4. 在 `src/App.vue` 从 `displayAssignments` 计算显示目标摘要，不再只依赖单一 `activeId`。

## 任务 3：菜单、弹窗与键盘/焦点契约

1. 写失败测试覆盖管理菜单和分类菜单的 Escape、外部点击、方向键/Home/End、触发器焦点返回。
2. 为确认弹窗补齐 `role="dialog"`、`aria-modal`、初始焦点、Tab 焦点环和关闭后的焦点恢复。
3. 异步提交期间禁用重复提交与破坏性关闭；取消时不修改草稿、选择或播放状态。
4. 使用组件内最小实现，不新增焦点管理依赖。

## 任务 4：详情/设置异步状态与后端分类约束

1. 写详情和设置失败测试，覆盖请求中禁用、防重复、失败保留当前值与可重试。
2. 在相关 Vue 页面增加局部 busy 状态，不做乐观写入。
3. 在 Rust 测试中先覆盖分类名 1–40 个字符边界和 41 字符拒绝，再在
   `normalized_category_name` 使用 Unicode 字符数校验。
4. 运行前端和 Rust 定向测试，确认错误码与中文提示稳定。

## 任务 5：视觉、契约与交付收口

1. 对照 Figma 节点 `93:422`、`180:137`、`100:874`、`144:3780`、`105:1395`、`105:1531`
   检查库、显示面板、批量栏、视频详情和图片详情。
2. 运行 `npm test -- --run`、`npm run build`、`cargo fmt -- --check`、`cargo test` 和 `git diff --check`。
3. 运行应用并截取关键状态，更新实现报告、契约矩阵和证据链接。
4. 仅提交本功能相关文件，提交信息使用中文 Conventional Commit。
