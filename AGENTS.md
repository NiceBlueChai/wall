<!-- Wall 仓库中自动化开发代理必须遵循的项目级约束。 -->

# Agent Instructions

本文件适用于整个仓库。完整的环境、格式、验证和提交规则以 [CONTRIBUTING.md](CONTRIBUTING.md) 为准。

## 工作边界

- 保持 Windows 10/11 x64、本地媒体和完全离线的产品边界；不得擅自增加账号、遥测或网络请求。
- 壁纸库移除、失效清理和分类操作只修改应用记录，不得删除、移动或修改用户源媒体。
- 前端和托盘复用现有 Rust 服务层与状态机，不维护第二套播放状态。
- UI、UX 或交互契约变化先对齐现有 Figma 文件和 `docs/design/`，确认后再同步代码。
- 优先复用现有组件、CSS 变量、类型和命令；没有明确必要时不增加依赖或抽象。
- 保留并绕开与当前任务无关的工作区改动。

## 验证与交付

- 前端变更至少运行 `npm test` 和 `npm run build`。
- Rust 变更至少运行 `cargo fmt --manifest-path src-tauri\Cargo.toml --all -- --check` 和
  `cargo test --manifest-path src-tauri\Cargo.toml`。
- 涉及窗口、托盘、桌面嵌入或系统事件时，补充 Windows 实机验证。
- 提交信息使用中文 Conventional Commit；用户可见行为或边界变化同步更新对应设计、实现或版本文档。
