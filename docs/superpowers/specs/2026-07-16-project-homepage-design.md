<!-- Wall 关于页打开项目主页的交互与实现约定。 -->

# 项目主页打开方案

## 目标

关于页的“打开项目主页”按钮启用后，将 `https://github.com/NiceBlueChai/wall` 交给 Windows 默认浏览器。
Wall 不内嵌网页，也不自行发送 HTTP 请求。

## 实现

- Vue 继续调用现有的 `openProjectHomepage()` API，并移除按钮的禁用状态。
- Rust 的 `open_project_homepage` 命令使用现有 Tauri opener 插件打开固定仓库地址。
- 系统无法启动默认浏览器时返回可恢复错误，由现有界面错误提示展示。
- 不新增配置项、依赖或第二套外部链接打开逻辑。

## 验证

- 组件测试证明按钮可点击并调用现有 API。
- Rust 测试固定项目主页地址，避免发布包指向错误位置。
- 设计规范、实现契约和中英文 README 同步说明该行为。
