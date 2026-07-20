<!-- 壁纸库移除与失效清理功能的跨层实施计划。 -->

# 壁纸库管理功能 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现单项、批量和失效项移出壁纸库的完整能力，始终保留源文件，并停止被移除壁纸关联的播放目标。

**Architecture:** Rust `WallCore` 统一拥有重新扫描与原子批量移除语义，Tauri 命令负责持久化、广播及停止播放器。
Vue API 只消费完整 `AppSnapshot`；壁纸库和详情页分别管理确认弹窗，共享 store 中的短时 Toast。

**Tech Stack:** Rust 2024、Tauri 2、Vue 3、TypeScript、Vitest、Cargo test。

## Global Constraints

- 所有移除操作只删除 Wall 的库记录，不删除、移动或修改源文件。
- 单项、批量和失效清理都必须确认；取消不得改变选择、播放或壁纸库状态。
- 清理确认前扫描全库，最终执行时再次扫描，不依赖旧的 `missing` 值。
- 移除运行中壁纸时允许继续，但必须停止所有相关播放目标。
- 复用现有图标、按钮、Dialog、Toast、颜色变量和中文文案，不新增依赖。

---

### Task 1: Rust 核心扫描与原子批量移除

**Files:**

- Modify: `src-tauri/tests/core_test.rs`
- Modify: `src-tauri/src/core.rs`

**Interfaces:**

- Produces: `WallCore::refresh_missing(&mut self) -> Vec<String>`
- Produces: `WallCore::remove_many(&mut self, ids: &[String]) -> Result<(), AppError>`
- Preserves: `WallCore::remove(&mut self, id: &str) -> Result<(), AppError>`

- [x] **Step 1: Write failing core tests**

```rust
#[test]
fn refresh_missing_rechecks_every_library_path() {
    // Import two real files, delete one, assert refresh_missing returns only it;
    // recreate the file and assert a second scan clears the stale missing flag.
}

#[test]
fn batch_remove_is_atomic_and_preserves_source_files() {
    // Include an unknown id and assert no library entry changes, then remove two valid ids,
    // assert their assignments stop while the source files remain on disk.
}
```

- [x] **Step 2: Run the focused test and verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test core_test refresh_missing_rechecks_every_library_path`

Expected: FAIL because `WallCore::refresh_missing` does not exist.

- [x] **Step 3: Implement the minimum shared core behavior**

```rust
pub fn refresh_missing(&mut self) -> Vec<String> {
    for item in &mut self.snapshot.library {
        item.missing = !Path::new(&item.path).is_file();
    }
    self.snapshot
        .library
        .iter()
        .filter(|item| item.missing)
        .map(|item| item.id.clone())
        .collect()
}

pub fn remove_many(&mut self, ids: &[String]) -> Result<(), AppError> {
    // Reject an empty or partially unknown request before mutating; then remove records and assignments.
}
```

- [x] **Step 4: Run both focused tests and verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test core_test refresh_missing`

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test core_test batch_remove`

Expected: both commands PASS.

### Task 2: Tauri 命令与前端 API 契约

**Files:**

- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/api.test.ts`
- Modify: `src/api.ts`

**Interfaces:**

- Produces: Tauri commands `scan_library`, `remove_media_batch`, `remove_missing_media`
- Produces: `scanLibrary()`, `removeMediaBatch(mediaIds)`, `removeMissingMedia()`

- [x] **Step 1: Write the failing API command test**

```typescript
await scanLibrary();
await removeMediaBatch(['video-1', 'video-2']);
await removeMissingMedia();
expect(invokeMock).toHaveBeenNthCalledWith(1, 'scan_library', undefined);
expect(invokeMock).toHaveBeenNthCalledWith(2, 'remove_media_batch', {
    mediaIds: ['video-1', 'video-2'],
});
expect(invokeMock).toHaveBeenNthCalledWith(3, 'remove_missing_media', undefined);
```

- [x] **Step 2: Run the API test and verify RED**

Run: `npm test -- src/api.test.ts`

Expected: FAIL because the three functions are not exported.

- [x] **Step 3: Add command wrappers and register them**

```typescript
export const scanLibrary = () => snapshotCommand('scan_library');
export const removeMediaBatch = (mediaIds: string[]) => snapshotCommand('remove_media_batch', { mediaIds });
export const removeMissingMedia = () => snapshotCommand('remove_missing_media');
```

Rust commands call `refresh_missing` or `remove_many` on a candidate `WallCore`. They replace shared state only after storage
succeeds, stop each removed media in `MpvPlayerManager`, and then publish the committed snapshot.

- [x] **Step 4: Run API and Rust tests and verify GREEN**

Run: `npm test -- src/api.test.ts`

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: all tests PASS.

### Task 3: Shared activity and Toast state

**Files:**

- Modify: `src/store.test.ts`
- Modify: `src/store.ts`
- Modify: `src/App.test.ts`
- Modify: `src/App.vue`

**Interfaces:**

- Produces: `wallStore.isMediaActive(mediaId)`
- Produces: `wallStore.showNotice(message)` and `wallStore.clearNotice()`

- [x] **Step 1: Write failing store and shell tests**

```typescript
expect(store.isMediaActive('video-1')).toBe(true);
store.showNotice('已从壁纸库移除');
expect(store.notice).toBe('已从壁纸库移除');
```

App shell test mounts `App.vue`, calls `wallStore.showNotice`, and asserts `.success-toast` renders the message.

- [x] **Step 2: Run focused tests and verify RED**

Run: `npm test -- src/store.test.ts src/App.test.ts`

Expected: FAIL because activity and notice APIs do not exist.

- [x] **Step 3: Implement shared state and render the existing Toast surface**

Store activity checks `displayAssignments` first and falls back to legacy `activeId`. Notice state auto-clears after 3000 ms;
`App.vue` renders it through the existing `.toast` layout without introducing a new component.

- [x] **Step 4: Run focused tests and verify GREEN**

Run: `npm test -- src/store.test.ts src/App.test.ts`

Expected: both files PASS.

### Task 4: 壁纸库管理、失效清理和批量移出

**Files:**

- Modify: `src/LibraryView.test.ts`
- Modify: `src/views/LibraryView.vue`
- Modify: `src/App.test.ts`
- Modify: `src/App.vue`
- Modify: `src/styles.css`

**Interfaces:**

- Consumes: `scanLibrary`, `removeMediaBatch`, `removeMissingMedia`, `wallStore.isMediaActive`
- Produces: header management menu, cleanup confirmation, batch removal confirmation

- [x] **Step 1: Write failing rendered-interaction tests**

Tests assert the management menu enters batch mode, cleanup scans before opening, zero results show a Toast, cleanup and batch
dialogs show counts/source-file copy/active warning, cancel preserves selection, and confirm calls the matching command once.

- [x] **Step 2: Run the library and shell tests and verify RED**

Run: `npm test -- src/LibraryView.test.ts src/App.test.ts`

Expected: FAIL because the new controls and dialogs are absent.

- [x] **Step 3: Implement the minimum confirmed Figma flow**

`LibraryView.vue` adds a `管理` dropdown beside display/import controls. Batch mode adds a separated icon-plus-text `移出库`
action. Both dialogs reuse `.modal-scrim`, `.dialog`, `.dialog-actions`; the final confirm button alone uses `.danger`.
The category sidebar keeps category create/rename/delete and no longer duplicates the batch entry.

- [x] **Step 4: Run the library and shell tests and verify GREEN**

Run: `npm test -- src/LibraryView.test.ts src/App.test.ts`

Expected: both files PASS.

### Task 5: 详情页单项移出

**Files:**

- Modify: `src/DetailView.test.ts`
- Modify: `src/views/DetailView.vue`
- Modify: `src/styles.css`

**Interfaces:**

- Consumes: `removeMedia(mediaId)` and `wallStore.isMediaActive(mediaId)`
- Produces: detail header danger action and single-item confirmation dialog

- [x] **Step 1: Write the failing detail interaction test**

The test opens the dialog, verifies item name/source-file copy/active warning, cancels without an API call, reopens, confirms,
asserts `removeMedia('video-1')`, success Toast state, and navigation back to `/`.

- [x] **Step 2: Run the detail test and verify RED**

Run: `npm test -- src/DetailView.test.ts`

Expected: FAIL because the removal action is absent.

- [x] **Step 3: Implement the confirmed single-item flow**

Add the header action and dialog using existing button, icon, warning and dialog styles. Close and navigate only after the Tauri
command succeeds; on error keep the dialog visible and use the existing inline error path.

- [x] **Step 4: Run the detail test and verify GREEN**

Run: `npm test -- src/DetailView.test.ts`

Expected: PASS.

### Task 6: Contract report and final gates

**Files:**

- Modify: `docs/implementation/wall-v1-contract-report.md`

**Interfaces:**

- Consumes: final command names, UI behavior, test counts and real/stubbed boundaries
- Produces: durable cross-layer contract evidence

- [x] **Step 1: Update the owner matrix and management contract**

Record Rust as required enforcement/state, Vue as required presentation, tests/report as required proof, and state explicitly that
no source-file deletion API exists in any new path.

- [x] **Step 2: Run formatting and all verification gates**

Run: `npx prettier --write src docs/superpowers/plans/2026-07-20-wallpaper-library-management.md`

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Run: `npm test`

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Run: `npm run build`

Expected: formatting exits 0; all tests and production build PASS.
