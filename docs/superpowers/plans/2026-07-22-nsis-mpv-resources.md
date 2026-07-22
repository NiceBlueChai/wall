<!-- 指导 Wall NSIS 安装包补齐视频播放依赖并重新生成发布产物。 -->

# NSIS Video Playback Resources Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 Wall v1.0.0 的 NSIS 安装包与便携包都完整携带视频播放所需的 mpv 运行文件。

**Architecture:** 复用 Tauri 原生 `bundle.resources` 映射，把仓库现有 `src-tauri/binaries` 中的两个文件安装到 `Wall.exe` 同目录。现有 Rust 查找逻辑保持不变，发布后通过归档内容检查证明安装包和便携包均完整。

**Tech Stack:** Tauri 2.11.4、NSIS、PowerShell、7-Zip、Rust、Vue 3

## Global Constraints

- 仅支持 Windows 10/11 x64、本地媒体和完全离线运行。
- 不增加依赖、网络请求、账号或遥测。
- 不修改播放器运行时逻辑和用户源媒体。
- `mpv.exe` 与 `d3dcompiler_43.dll` 必须安装到 `Wall.exe` 同目录。
- 配置文件保持 UTF-8 无 BOM、CRLF 和 4 空格缩进。
- 安装包代码签名不在本次范围内。

---

### Task 1: 补齐安装资源并重建发布包

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Verify: `src-tauri/target/release/bundle/nsis/Wall_1.0.0_x64-setup.exe`
- Verify: `release/Wall-v1.0.0-windows-x64-portable.zip`

**Interfaces:**
- Consumes: `src-tauri/binaries/mpv.exe`、`src-tauri/binaries/d3dcompiler_43.dll` 和现有 `resolve_mpv_binary()` 查找规则。
- Produces: 安装后与 `Wall.exe` 同目录的 `mpv.exe`、`d3dcompiler_43.dll`，以及完整的 NSIS 与便携发布包。

- [ ] **Step 1: 复现当前安装包缺少资源**

Run:

```powershell
$installer = "src-tauri\target\release\bundle\nsis\Wall_1.0.0_x64-setup.exe"
$listing = (& 7z l $installer) -join "`n"
foreach ($name in @("mpv.exe", "d3dcompiler_43.dll")) {
    if ($listing -notmatch [regex]::Escape($name)) {
        throw "NSIS package is missing $name"
    }
}
```

Expected: FAIL with `NSIS package is missing mpv.exe`.

- [ ] **Step 2: 添加最小 Tauri 资源映射**

Modify `src-tauri/tauri.conf.json` so the bundle section is:

```json
"bundle": {
    "active": true,
    "targets": ["nsis"],
    "resources": {
        "binaries/mpv.exe": "mpv.exe",
        "binaries/d3dcompiler_43.dll": "d3dcompiler_43.dll"
    },
    "shortDescription": "完全离线的 Windows 动态壁纸工具"
}
```

- [ ] **Step 3: 运行源码验证**

Run:

```powershell
npm test
npm run build
cargo fmt --manifest-path src-tauri\Cargo.toml --all -- --check
cargo test --manifest-path src-tauri\Cargo.toml
```

Expected: 前端 63 项测试通过，构建成功；Rust 54 项测试通过，格式检查成功。

- [ ] **Step 4: 重新生成 NSIS 安装包**

Run:

```powershell
& .\node_modules\.bin\tauri.cmd build --bundles nsis
```

Expected: 成功生成 `src-tauri/target/release/bundle/nsis/Wall_1.0.0_x64-setup.exe`。

- [ ] **Step 5: 验证 NSIS 包含完整运行文件**

Run:

```powershell
$installer = "src-tauri\target\release\bundle\nsis\Wall_1.0.0_x64-setup.exe"
$listing = (& 7z l $installer) -join "`n"
foreach ($name in @("wall.exe", "mpv.exe", "d3dcompiler_43.dll")) {
    if ($listing -notmatch [regex]::Escape($name)) {
        throw "NSIS package is missing $name"
    }
}
```

Expected: PASS with exit code 0.

- [ ] **Step 6: 重新生成并验证便携包**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\package-portable.ps1
7z t release\Wall-v1.0.0-windows-x64-portable.zip
```

Expected: 打包脚本与 7-Zip 完整性检查均以 exit code 0 结束。

- [ ] **Step 7: 检查差异并提交配置修复**

Run:

```powershell
git diff --check
git status --short
git add src-tauri/tauri.conf.json
git commit -m "fix(release): 补齐安装包视频播放依赖"
```

Expected: 只提交 `src-tauri/tauri.conf.json`，发布产物继续由 `.gitignore` 排除。

- [ ] **Step 8: 输出发布哈希与工作区状态**

Run:

```powershell
$artifacts = @(
    "src-tauri\target\release\bundle\nsis\Wall_1.0.0_x64-setup.exe",
    "release\Wall-v1.0.0-windows-x64-portable.zip"
)
Get-FileHash -Algorithm SHA256 $artifacts
git status --short --branch
```

Expected: 两个文件均有 SHA-256，工作区无未提交文件。
