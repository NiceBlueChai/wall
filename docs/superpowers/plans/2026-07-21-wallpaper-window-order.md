# 视频壁纸窗口层级稳定性 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让多个视频壁纸窗口由一个稳定的桌面层级维护线程管理，消除复制模式露出 Windows 背景的闪动。

**Architecture:** `MpvPlayer` 只保存自身窗口与 raised desktop 图标层句柄；`MpvPlayerManager` 汇总全部活动窗口，
构造稳定的父层级链并只启动一个维护线程。播放器集合改变时重建维护线程，停止时先结束线程再销毁窗口。

**Tech Stack:** Rust 2024、windows-rs Win32 API、现有 Cargo 单元测试。

## Global Constraints

- 不修改 Vue、Figma、显示模式数据结构或 mpv 数量。
- 不增加依赖，不扫描或终止系统中的任意 mpv 进程。
- Rust 源码保持 UTF-8、CRLF、4 空格缩进，单行不超过 120 字符。
- 只在 Windows raised desktop 路径维护窗口层级；其他路径保持现状。

---

### Task 1: 统一播放器窗口层级所有权

**Files:**

- Modify: `src-tauri/src/player.rs:203-211`
- Modify: `src-tauri/src/player.rs:780-890`
- Modify: `src-tauri/src/player.rs:1142-1376`
- Test: `src-tauri/src/player.rs:1378-1580`

**Interfaces:**

- Consumes: `MpvPlayer.player_window: Option<isize>`、`DesktopHost.shell_view: isize`、现有 `SetWindowPos`。
- Produces: `window_order_chain(Vec<(isize, isize)>) -> Vec<(isize, isize)>`，输入元组为
  `(player_window, shell_view)`，输出元组为 `(player_window, shell_or_previous_player)`；
  `MpvPlayerManager::refresh_window_order(&mut self)`。

- [ ] **Step 1: 写入失败测试**

在 `player.rs` 的单元测试模块导入 `window_order_chain`，添加：

```rust
#[test]
fn multiple_windows_form_one_stable_desktop_chain() {
    assert_eq!(
        window_order_chain(vec![(30, 7), (20, 7)]),
        vec![(20, 7), (30, 20)]
    );
}
```

- [ ] **Step 2: 确认测试因缺少稳定链实现而失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml multiple_windows_form_one_stable_desktop_chain`

Expected: FAIL，提示无法导入或找到 `window_order_chain`。

- [ ] **Step 3: 实现最小稳定链与单一维护线程**

在 `player.rs` 中完成以下变更：

```rust
fn window_order_chain(mut targets: Vec<(isize, isize)>) -> Vec<(isize, isize)> {
    targets.sort_unstable_by_key(|(player, shell)| (*shell, *player));
    let mut previous = None;
    targets
        .into_iter()
        .map(|(player, shell)| {
            let insert_after = match previous {
                Some((previous_shell, previous_player)) if previous_shell == shell => {
                    previous_player
                }
                _ => shell,
            };
            previous = Some((shell, player));
            (player, insert_after)
        })
        .collect()
}
```

`MpvPlayer` 删除每实例 `order_stop` / `order_thread`，增加 `shell_view: Option<isize>`；嵌入 raised desktop 后保存
`shell_view`，停止时清空。`MpvPlayerManager` 增加唯一的 `order_stop` / `order_thread`，并实现：

```rust
fn refresh_window_order(&mut self) {
    self.stop_window_order();
    let targets = self
        .groups
        .values()
        .flat_map(|group| &group.players)
        .filter_map(|player| Some((player.player_window?, player.shell_view?)))
        .collect::<Vec<_>>();
    if !targets.is_empty() {
        let (stop, thread) = maintain_window_order(targets);
        self.order_stop = Some(stop);
        self.order_thread = Some(thread);
    }
}
```

将 `maintain_window_order` 改为接收全部目标，并对 `window_order_chain(targets)` 逐项调用
`ensure_window_order(player, insert_after)`。在组插入、目标停止、媒体停止后刷新；全部停止时先结束维护线程。

- [ ] **Step 4: 确认定向测试与播放器测试通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml player::tests`

Expected: PASS，包含 `multiple_windows_form_one_stable_desktop_chain`。

- [ ] **Step 5: 运行回归闭环**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: 所有 Rust 测试 PASS。

Run: `npm test`

Expected: 所有前端测试 PASS。

Run: `npm run build`

Expected: TypeScript 与 Vite 构建成功。

Run: `.\\node_modules\\.bin\\tauri.cmd build --no-bundle`

Expected: release 可执行文件生成到 `src-tauri/target/release/wall.exe`。

- [ ] **Step 6: 双屏实机验证**

精确确认并结束当前遗留的 `D:\\projects\\github\\wall\\src-tauri\\binaries\\mpv.exe` 孤儿 PID；运行新 release，
选择两块在线显示器的复制模式并播放视频。连续观察至少 15 秒，预期两个屏幕均持续显示视频，不露出 Windows
背景；停止壁纸并退出应用后，不应留下本次启动的 mpv 进程。

- [ ] **Step 7: 提交实现**

```powershell
git add -- src-tauri/src/player.rs
git commit -m "fix(player): 稳定多屏视频壁纸窗口层级"
```
