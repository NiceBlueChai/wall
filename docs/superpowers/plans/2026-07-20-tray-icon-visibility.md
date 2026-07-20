<!-- 托盘图标体量与状态辨识优化的逐步实施计划。 -->

# 托盘图标可见性实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 Figma 与 Windows 通知区中的三态托盘图标从 20 × 20 内容放大到 22 × 22，并强化运行/暂停角标。

**Architecture:** 保留现有 `AppSnapshot -> TrayIconState -> Tauri TrayIcon` 数据流，只替换三个现有 Figma
组件的图形内容和嵌入式 RGBA 资源。用一个 Rust 资源边界测试固定 22 × 22 非透明边界，避免以后再次导出偏小图标。

**Tech Stack:** Figma Plugin API、Tauri 2、Rust、24 × 24 PNG/RGBA。

## Global Constraints

- 保留 `Active`、`Paused`、`Idle` 三态，不增加状态、配置或依赖。
- 画布保持 24 × 24，主体边界统一为 22 × 22，四周保留 1px 安全边距。
- 直接修改 Figma 节点 `10:191`、`10:199`、`10:205`，不新增组件。
- 不修改应用主图标、安装包图标、任务栏窗口图标或播放状态模型。
- 源文件保持 UTF-8 无 BOM、CRLF、4 空格缩进和不超过 120 字符的源码行。

---

### Task 1: 修改现有 Figma 三态组件

**Files:**
- Modify: Figma `SdCRz7VSCFwVC3Apk9dniq`, nodes `10:191`, `10:199`, `10:205`

**Interfaces:**
- Consumes: 现有三个 24 × 24 `Tray / Active|Paused|Idle` 组件 ID。
- Produces: 保持原组件 ID 的 22 × 22 主体和约 7px 状态角标。

- [x] **Step 1: 截取修改前三个组件**

调用 Figma `get_screenshot`，分别记录 `10:191`、`10:199`、`10:205` 的 24 × 24 当前状态。

- [x] **Step 2: 原位替换三个组件的图形内容**

用 `use_figma` 对三个现有组件执行以下脚本；脚本保留组件节点本身，只替换其子图形：

```js
const page = await figma.getNodeByIdAsync("5:6");
if (!page || page.type !== "PAGE") throw new Error("Expected 04 Icons page 5:6");
await figma.setCurrentPageAsync(page);

const specs = [
  {
    id: "10:191",
    svg: `<svg width="24" height="24" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
      <rect x="1" y="1" width="22" height="22" rx="5" fill="#7C5CFF"/>
      <path d="M5.2 7.7 8.8 17l3.2-7.1 3.2 7.1 3.6-9.3" fill="none" stroke="#FFFFFF"
        stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round"/>
      <circle cx="19.2" cy="5.1" r="3.6" fill="#12151D"/>
      <circle cx="19.2" cy="5.1" r="2.7" fill="#35D07F"/>
    </svg>`,
  },
  {
    id: "10:199",
    svg: `<svg width="24" height="24" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
      <rect x="1" y="1" width="22" height="22" rx="5" fill="#7C5CFF"/>
      <path d="M5.2 7.7 8.8 17l3.2-7.1 3.2 7.1 3.6-9.3" fill="none" stroke="#FFFFFF"
        stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round"/>
      <circle cx="19.2" cy="5.1" r="3.6" fill="#12151D"/>
      <circle cx="19.2" cy="5.1" r="2.7" fill="#FFBD45"/>
      <path d="M18.3 3.8v2.6m1.8-2.6v2.6" stroke="#4B3100" stroke-width="0.7"
        stroke-linecap="round"/>
    </svg>`,
  },
  {
    id: "10:205",
    svg: `<svg width="24" height="24" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
      <rect x="1" y="1" width="22" height="22" rx="5" fill="#4B5063"/>
      <path d="M5.2 7.7 8.8 17l3.2-7.1 3.2 7.1 3.6-9.3" fill="none" stroke="#AEB4C8"
        stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,
  },
];

const changed = [];
for (const spec of specs) {
  const component = await figma.getNodeByIdAsync(spec.id);
  if (!component || component.type !== "COMPONENT") {
    throw new Error(`Expected component ${spec.id}`);
  }
  const replacement = figma.createNodeFromSvg(spec.svg);
  for (const child of [...component.children]) child.remove();
  while (replacement.children.length > 0) component.appendChild(replacement.children[0]);
  replacement.remove();
  component.resize(24, 24);
  changed.push({ id: component.id, name: component.name, width: component.width, height: component.height });
}
return changed;
```

- [x] **Step 3: 截取修改后三个组件并检查**

再次调用 Figma `get_screenshot`。预期：三个节点仍为 24 × 24；主体一致；运行角标为绿色、暂停角标为黄色且
带暂停符号；空闲态对比更清楚但不抢眼。

---

### Task 2: 导出资源并增加边界回归测试

**Files:**
- Modify: `src-tauri/icons/tray/active.png`
- Modify: `src-tauri/icons/tray/paused.png`
- Modify: `src-tauri/icons/tray/idle.png`
- Modify: `src-tauri/icons/tray/active.rgba`
- Modify: `src-tauri/icons/tray/paused.rgba`
- Modify: `src-tauri/icons/tray/idle.rgba`
- Modify: `src-tauri/src/tray.rs`

**Interfaces:**
- Consumes: Task 1 的三个 24 × 24 Figma 组件。
- Produces: Tauri 可直接嵌入的三个 2304-byte RGBA 资源和 22 × 22 边界测试。

- [x] **Step 1: 先写会失败的资源边界测试**

在 `src-tauri/src/tray.rs` 的现有测试模块增加：

```rust
fn opaque_bounds(bytes: &[u8]) -> (usize, usize, usize, usize) {
    let mut min_x = 24;
    let mut min_y = 24;
    let mut max_x = 0;
    let mut max_y = 0;
    for (index, pixel) in bytes.chunks_exact(4).enumerate() {
        if pixel[3] <= 16 {
            continue;
        }
        let x = index % 24;
        let y = index / 24;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    (min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
}
```

并在 `tray_icon_tracks_idle_paused_and_active_playback` 末尾增加：

```rust
assert_eq!(opaque_bounds(TrayIconState::Active.bytes()), (1, 1, 22, 22));
assert_eq!(opaque_bounds(TrayIconState::Paused.bytes()), (1, 1, 22, 22));
assert_eq!(opaque_bounds(TrayIconState::Idle.bytes()), (1, 1, 22, 22));
```

- [x] **Step 2: 运行测试并确认旧资源失败**

Run: `cargo test tray_icon_tracks_idle_paused_and_active_playback --manifest-path src-tauri/Cargo.toml`

Expected: FAIL，旧资源边界为 `(2, 2, 20, 20)`。

- [x] **Step 3: 导出并替换 PNG**

对 Task 1 的三个节点调用 Figma `get_screenshot`，将短期 PNG URL 分别下载到上述三个 `.png` 文件。每个文件
必须是 24 × 24 PNG。

- [x] **Step 4: 用系统图像库机械生成 RGBA**

Run:

```powershell
Add-Type -AssemblyName System.Drawing
Get-ChildItem 'src-tauri\icons\tray\*.png' | ForEach-Object {
    $trayBitmap = [System.Drawing.Bitmap]::new($_.FullName)
    $trayPixels = [System.Collections.Generic.List[byte]]::new(2304)
    for ($trayY = 0; $trayY -lt 24; $trayY++) {
        for ($trayX = 0; $trayX -lt 24; $trayX++) {
            $trayColor = $trayBitmap.GetPixel($trayX, $trayY)
            $trayPixels.Add($trayColor.R)
            $trayPixels.Add($trayColor.G)
            $trayPixels.Add($trayColor.B)
            $trayPixels.Add($trayColor.A)
        }
    }
    $trayBitmap.Dispose()
    $trayOutput = [System.IO.Path]::ChangeExtension($_.FullName, '.rgba')
    [System.IO.File]::WriteAllBytes($trayOutput, $trayPixels.ToArray())
}
```

Expected: 三个 `.rgba` 文件均为 2304 bytes。

- [x] **Step 5: 运行目标测试**

Run: `cargo test tray_icon_tracks_idle_paused_and_active_playback --manifest-path src-tauri/Cargo.toml`

Expected: PASS。

---

### Task 3: 回归与原生检查

**Files:**
- Modify: `docs/implementation/wallpaper-library-management-report.md`

**Interfaces:**
- Consumes: Task 2 的三态资源和测试。
- Produces: 完整回归结果、Figma 三态截图和 Windows 通知区人工结论。

- [x] **Step 1: 运行格式与完整 Rust 测试**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Expected: PASS。

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: 现有测试和新增资源边界测试全部 PASS。

- [ ] **Step 2: 重启开发应用并人工切换三态**

Run: `npm run tauri dev`

依次观察：无播放目标为灰色 Idle；播放目标运行时为绿色角标 Active；全部暂停时为黄色角标 Paused。

- [x] **Step 3: 更新实现报告**

在报告中记录根因 `20 × 20 内容被 Windows 继续缩小`、修复层级 `class repair`、测试命令和三态视觉证据；若
系统托盘无法截图，明确记录原生视觉验证缺口，不用浏览器截图替代。

- [x] **Step 4: 提交相关文件**

```powershell
git add docs/implementation/wallpaper-library-management-report.md `
    src-tauri/src/tray.rs src-tauri/icons/tray
git commit -m "fix(tray): 增强托盘图标状态辨识度"
```
