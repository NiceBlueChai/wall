# mpv Hot Switch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reuse compatible mpv target processes and switch local wallpaper media through JSON IPC without recreating the
desktop window.

**Architecture:** `MpvPlayerManager` first checks whether the target's process contract is unchanged. Compatible groups
send checked `loadfile ... replace` requests to their existing players, reapply live settings, and update group metadata;
incompatible or unreachable groups fall through to the existing atomic restart path.

**Tech Stack:** Rust, Tauri 2, mpv JSON IPC, Windows WorkerW, Cargo tests, PowerShell Windows UI Automation.

## Global Constraints

- Windows 10/11 x64, completely offline, no new dependency or network request.
- Reuse only when target, mode, displays, geometry, media kind, and hardware decoding are compatible.
- Video/image cross-kind switches and hardware decoding changes keep the existing restart behavior.
- UTF-8 without BOM, CRLF, four-space indentation, Rust source lines at most 120 characters.
- UI, Figma, command names, JSON persistence, and error payloads remain unchanged.

---

### Task 1: Define the hot-switch contract

**Files:**
- Modify: `src-tauri/src/player.rs`
- Test: `src-tauri/src/player.rs` internal test module

**Interfaces:**
- Consumes: existing `PlayerGroup`, `ScreenRegion`, `MediaKind`, `DisplayMode`, and `AppSettings`.
- Produces: private `can_hot_switch(...) -> bool` and `loadfile_command(path, request_id) -> Value` helpers.

- [ ] **Step 1: Write failing tests for compatibility and the checked load command**

Add internal tests that construct a one-player group and prove identical video contracts are reusable while media kind,
hardware decoding, display mode, display IDs, or regions make the contract incompatible. Assert the command is exactly:

```rust
json!({
    "command": ["loadfile", r"D:\Wallpapers\next.mp4", "replace"],
    "request_id": 2
})
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml player::tests::hot_switch -- --nocapture`

Expected: compilation fails because `can_hot_switch` and `loadfile_command` do not exist.

- [ ] **Step 3: Implement the minimum pure helpers**

`can_hot_switch` must compare the existing binary, kind, hardware decoding, mode, display IDs, source regions, and player
count against the computed target regions. `loadfile_command` must preserve the Windows path as a JSON string and include
the request ID.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml player::tests::hot_switch -- --nocapture`

Expected: both focused tests pass.

### Task 2: Reuse compatible player groups

**Files:**
- Modify: `src-tauri/src/player.rs`
- Test: `src-tauri/src/player.rs` internal test module and `src-tauri/tests/player_test.rs`

**Interfaces:**
- Consumes: `can_hot_switch`, `loadfile_command`, existing `request`, `apply_settings`, and restart path.
- Produces: private `MpvPlayer::load_media(&self, path: &Path) -> Result<(), PlayerError>` and
  `MpvPlayerManager::try_hot_switch(...) -> Result<bool, PlayerError>`.

- [ ] **Step 1: Write a failing response-validation test**

Extract a pure response check and assert `{"error":"success"}` succeeds while `{"error":"loading failed"}` returns a
player command error containing `loading failed`.

- [ ] **Step 2: Run the focused test and verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml player::tests::loadfile_response -- --nocapture`

Expected: compilation fails because the response validator and player command error do not exist.

- [ ] **Step 3: Implement checked media loading and manager reuse**

Add one `PlayerError` variant for an mpv-rejected IPC command. `load_media` sends `loadfile` through `request` and validates
the response. `try_hot_switch` removes the target group temporarily, verifies compatibility, loads every player, reapplies
live settings, restores clone synchronization and final pause/mute/volume, updates group metadata, and reinserts it.

On failure, best-effort reload the old media and old settings. IPC/child-process failures reinsert the group and return
`Ok(false)` so the existing new-process path can replace it; an explicit mpv command rejection returns the error after
rollback. `play_target_configured` calls this method before spawning any process.

- [ ] **Step 4: Run Rust tests and verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: all existing tests plus the new hot-switch tests pass.

- [ ] **Step 5: Run formatting and lint checks**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`

Expected: both commands exit 0.

### Task 3: Close the runtime contract and release proof

**Files:**
- Modify: `CHANGELOG.md`
- Modify: `docs/implementation/wall-v1-contract-report.md`
- Modify generated artifact: `release/Wall-v1.0.0-windows-x64-portable.zip`

**Interfaces:**
- Consumes: compatible-process hot switch from Task 2.
- Produces: durable contract evidence and rebuilt portable release.

- [ ] **Step 1: Update behavior documentation**

Record that compatible same-kind wallpaper switches reuse the mpv PID, while kind, hardware decoding, or layout changes
restart. Record visual proof as N/A because the UI is unchanged and native PID/timing evidence replaces a screenshot.

- [ ] **Step 2: Run full repository verification**

Run: `npm test`

Run: `npm run build`

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: 20 frontend tests and all Rust tests pass; Vite production build exits 0.

- [ ] **Step 3: Rebuild the portable package**

Run: `powershell -ExecutionPolicy Bypass -File scripts/package-portable.ps1`

Expected: `release/Wall-v1.0.0-windows-x64-portable.zip` is recreated with Wall, mpv, documentation, and three MP4 samples.

- [ ] **Step 4: Verify the real release without mouse input**

Back up `%APPDATA%\com.wall.app\session.json`, start the release, and use Windows UI Automation to switch among the three
video cards. Assert the mpv PID remains unchanged for compatible switches, session commits complete without a long-tail
timeout, Wall remains responsive, and the original session file is restored byte-for-byte afterward.

- [ ] **Step 5: Review, commit, and push**

Run `git diff --check`, request a focused review, and fix every Critical/Important issue. Then commit with:

```text
fix(player): 优化视频壁纸切换延迟
```

Push the final commit to `origin/main` and verify `HEAD == origin/main`.
