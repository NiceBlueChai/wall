<!-- Wall 关于页项目主页功能的测试驱动实施清单。 -->

# Project Homepage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 启用关于页的项目主页按钮，并通过 Windows 默认浏览器打开 Wall 的 GitHub 仓库。

**Architecture:** Vue 保留现有 Tauri API 调用；Rust 命令使用已安装的 opener 插件把固定 URL 交给操作系统。
现有错误展示负责处理默认浏览器启动失败，不新增依赖、配置或网络客户端。

**Tech Stack:** Vue 3、Vitest、Tauri 2、Rust、tauri-plugin-opener。

## Global Constraints

- 项目主页固定为 `https://github.com/NiceBlueChai/wall`。
- 只有用户主动点击时才交给系统默认浏览器；Wall 自身不发送 HTTP 请求。
- UTF-8 无 BOM、CRLF、4 空格缩进，源码单行不超过 120 字符。
- 本改动规模小且共享文件集中，使用当前工作区串行实施，避免并行写入冲突。

---

### Task 1: 启用项目主页入口

**Files:**

- Modify: `src/SettingsView.test.ts`
- Modify: `src/views/SettingsView.vue`
- Modify: `src-tauri/src/commands.rs`
- Modify: `docs/design/wall-v1-design-spec.md`
- Modify: `docs/implementation/wall-v1-contract-report.md`
- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Modify: `CHANGELOG.md`

**Interfaces:**

- Consumes: `openProjectHomepage(): Promise<unknown>` 和 `tauri_plugin_opener::OpenerExt`。
- Produces: `open_project_homepage(app: AppHandle) -> Result<(), AppError>`。

- [x] **Step 1: 写入失败测试**

```typescript
expect(homepageButton.attributes('disabled')).toBeUndefined();
await homepageButton.trigger('click');
expect(mocks.openProjectHomepage).toHaveBeenCalledOnce();
```

```rust
#[test]
fn project_homepage_points_to_the_public_repository() {
    assert_eq!(PROJECT_HOMEPAGE, "https://github.com/NiceBlueChai/wall");
}
```

- [x] **Step 2: 运行测试并确认失败原因**

Run: `npm run test -- src/SettingsView.test.ts`
Expected: FAIL，因为按钮仍为禁用状态。

Run: `cargo test --manifest-path src-tauri/Cargo.toml project_homepage_points_to_the_public_repository`
Expected: FAIL，因为 `PROJECT_HOMEPAGE` 尚不存在。

- [x] **Step 3: 写入最小实现**

```rust
const PROJECT_HOMEPAGE: &str = "https://github.com/NiceBlueChai/wall";

#[tauri::command]
pub fn open_project_homepage(app: AppHandle) -> Result<(), AppError> {
    app.opener()
        .open_url(PROJECT_HOMEPAGE, None::<String>)
        .map_err(|problem| error("open_homepage_failed", &problem.to_string(), true))
}
```

移除 Vue 按钮的 `disabled` 属性，并把离线说明改为“点击后使用系统默认浏览器打开；Wall 自身不请求网络”。

- [x] **Step 4: 更新产品契约文档**

将项目主页从“未配置”改为已配置，说明真实 OS 浏览器边界、组件测试替身和无法启动浏览器时的可恢复错误。

- [x] **Step 5: 运行回归与发布验证**

Run: `npm run test`
Expected: 退出码 0。

Run: `npm run build`
Expected: 退出码 0。

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
Expected: 退出码 0。

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 退出码 0。

Run: `powershell -ExecutionPolicy Bypass -File scripts/package-portable.ps1`
Expected: 生成包含双语 README 和三个测试视频的便携 ZIP。

- [x] **Step 6: 提交并推送 `main`**

```powershell
git add --all
git commit -m "feat(about): 启用项目主页入口"
git branch -M main
git remote add origin git@github.com:NiceBlueChai/wall.git
git push -u origin main
```
