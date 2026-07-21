# Transparent Windows Icon Implementation Plan

<!-- 说明从现有 SVG 重建 Wall 透明 PNG 与多尺寸 ICO 的测试先行步骤。 -->

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 移除 Windows 任务栏图标的深色方形边缘，并提供多尺寸 ICO。

**Architecture:** 复用 `public/icons/app.svg`，用项目现有 Tauri CLI 在系统临时目录生成标准图标集。
只同步三个应用图标资源，不提交其他平台产物或修改托盘图标。

**Tech Stack:** Tauri CLI、SVG、PNG、ICO、PowerShell

## Global Constraints

- 保留现有紫色圆角 Wall 标志，不重新设计或使用 AI 重画。
- 不修改托盘状态图标、窗口布局或 Figma。
- 文件保持 UTF-8、CRLF、4 空格缩进，源码单行不超过 120 字符。
- 不新增依赖。

---

### Task 1: Rebuild transparent application icons

**Files:**

- Modify: `src-tauri/icons/icon.png`
- Modify: `src-tauri/icons/icon.ico`
- Modify: `public/wall-app-icon.png`

**Interfaces:**

- Consumes: `public/icons/app.svg`。
- Produces: 透明 PNG 和包含多个 Windows 尺寸图层的 ICO。

- [ ] **Step 1: Run the failing asset check**

运行以下 PowerShell 检查：

```powershell
Add-Type -AssemblyName System.Drawing
$bitmap = [System.Drawing.Bitmap]::FromFile((Resolve-Path 'src-tauri\icons\icon.png'))
$cornerAlpha = $bitmap.GetPixel(0, 0).A
$bitmap.Dispose()
$icoBytes = [System.IO.File]::ReadAllBytes((Resolve-Path 'src-tauri\icons\icon.ico'))
$icoCount = [BitConverter]::ToUInt16($icoBytes, 4)
if ($cornerAlpha -ne 0) { throw "PNG corner alpha is $cornerAlpha, expected 0" }
if ($icoCount -lt 4) { throw "ICO contains $icoCount image, expected at least 4" }
```

Expected: FAIL because the PNG corner Alpha is 255 and the ICO contains one image.

- [ ] **Step 2: Generate the standard icon set in a temporary directory**

```powershell
$generatedIconDir = Join-Path ([System.IO.Path]::GetTempPath()) ("wall-icon-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $generatedIconDir | Out-Null
.\node_modules\.bin\tauri.cmd icon 'public\icons\app.svg' --output $generatedIconDir
```

Expected: the temporary directory contains `icon.png` and `icon.ico` plus unused platform-specific assets.

- [ ] **Step 3: Copy only the required generated assets**

```powershell
Copy-Item -LiteralPath (Join-Path $generatedIconDir 'icon.png') -Destination 'src-tauri\icons\icon.png' -Force
Copy-Item -LiteralPath (Join-Path $generatedIconDir 'icon.ico') -Destination 'src-tauri\icons\icon.ico' -Force
Copy-Item -LiteralPath (Join-Path $generatedIconDir 'icon.png') -Destination 'public\wall-app-icon.png' -Force
```

- [ ] **Step 4: Verify pixels, ICO layers and visual output**

Repeat Step 1.

Expected: PASS with corner Alpha 0 and at least four ICO images.

Inspect `src-tauri/icons/icon.png` and the extracted 32 px ICO frame visually.

Expected: transparent corners, clean purple rounded-square outline, white W, no dark border.

- [ ] **Step 5: Run project verification**

Run: `npm test`

Expected: all tests PASS.

Run: `npm run build`

Expected: TypeScript checking and Vite build succeed.

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: Rust and Tauri configuration check succeeds.

- [ ] **Step 6: Remove temporary output and commit**

Confirm `$generatedIconDir` resolves beneath the system temporary directory, then delete it. Stage only the three generated resources and commit:

```powershell
git commit -m "fix(icon): 修复 Windows 任务栏图标黑边"
```
