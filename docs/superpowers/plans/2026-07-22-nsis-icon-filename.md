<!-- 指导 Wall 安装包使用应用图标并生成连字符公开文件名。 -->

# NSIS Installer Icon and Filename Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 Wall NSIS 安装包使用现有应用图标，并固定生成公开文件 `release/Wall-1.0.0-x64-setup.exe`。

**Architecture:** Tauri 配置负责把现有 ICO 嵌入安装器和卸载器；独立 PowerShell 脚本负责运行原生 NSIS 构建并把 Tauri 内部产物复制为连字符发布名。应用代码、图标像素和便携包流程保持不变。

**Tech Stack:** Tauri 2.11.4、NSIS、PowerShell、7-Zip、System.Drawing

## Global Constraints

- 仅支持 Windows 10/11 x64、本地媒体和完全离线运行。
- 不增加依赖、网络请求、账号或遥测。
- 复用 `src-tauri/icons/icon.ico` 和 `src-tauri/icons/icon.png`，不生成新图标。
- 公开安装包名称固定为 `release/Wall-1.0.0-x64-setup.exe`，不得包含下划线。
- 配置与脚本保持 UTF-8 无 BOM、CRLF 和 4 空格缩进。
- 安装包代码签名不在本次范围内。

---

### Task 1: 固化安装包图标与公开名称

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Create: `scripts/package-installer.ps1`
- Modify: `CONTRIBUTING.md`
- Verify: `release/Wall-1.0.0-x64-setup.exe`

**Interfaces:**
- Consumes: `src-tauri/icons/icon.ico`、`src-tauri/icons/icon.png`、`src-tauri/binaries/mpv.exe`、`src-tauri/binaries/d3dcompiler_43.dll` 和仓库本地 Tauri CLI。
- Produces: `scripts/package-installer.ps1` 发布入口与 `release/Wall-<version>-x64-setup.exe` 安装包。

- [ ] **Step 1: 运行失败断言证明配置和脚本尚未就绪**

Run:

```powershell
$config = Get-Content -Raw src-tauri\tauri.conf.json | ConvertFrom-Json
if ($config.bundle.windows.nsis.installerIcon -ne "icons/icon.ico") {
    throw "NSIS installer icon is not configured."
}
if (-not (Test-Path -LiteralPath scripts\package-installer.ps1)) {
    throw "Installer packaging script is missing."
}
```

Expected: FAIL with `NSIS installer icon is not configured.`

- [ ] **Step 2: 添加 Tauri 图标配置**

Modify `src-tauri/tauri.conf.json` so the bundle section includes:

```json
"icon": ["icons/icon.ico", "icons/icon.png"],
"windows": {
    "nsis": {
        "installerIcon": "icons/icon.ico",
        "uninstallerIcon": "icons/icon.ico"
    }
}
```

Keep the existing `targets`, `resources`, and `shortDescription` values unchanged.

- [ ] **Step 3: 创建安装版发布脚本**

Create `scripts/package-installer.ps1` with:

```powershell
<# Builds the Wall NSIS installer with its public release filename. #>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$ConfigPath = Join-Path $ProjectRoot "src-tauri\tauri.conf.json"
$Config = Get-Content -LiteralPath $ConfigPath -Raw | ConvertFrom-Json
$Version = $Config.version
$TauriCli = Join-Path $ProjectRoot "node_modules\.bin\tauri.cmd"
$Icon = Join-Path $ProjectRoot "src-tauri\icons\icon.ico"
$MpvDirectory = Join-Path $ProjectRoot "src-tauri\binaries"
$ReleaseRoot = Join-Path $ProjectRoot "release"
$InternalInstaller = Join-Path $ProjectRoot `
    "src-tauri\target\release\bundle\nsis\Wall_${Version}_x64-setup.exe"
$PublishedInstaller = Join-Path $ReleaseRoot "Wall-$Version-x64-setup.exe"

foreach ($file in @($TauriCli, $Icon, (Join-Path $MpvDirectory "mpv.exe"), `
    (Join-Path $MpvDirectory "d3dcompiler_43.dll"))) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        throw "Required installer input is missing: $file"
    }
}

New-Item -ItemType Directory -Force -Path $ReleaseRoot | Out-Null
foreach ($artifact in @($InternalInstaller, $PublishedInstaller)) {
    if (Test-Path -LiteralPath $artifact) {
        Remove-Item -LiteralPath $artifact -Force
    }
}

Push-Location $ProjectRoot
try {
    & $TauriCli build --bundles nsis
    if ($LASTEXITCODE -ne 0) {
        throw "The Tauri NSIS build failed."
    }
} finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath $InternalInstaller -PathType Leaf)) {
    throw "The Tauri NSIS build did not create $InternalInstaller."
}
Copy-Item -LiteralPath $InternalInstaller -Destination $PublishedInstaller
Write-Host "Installer package created: $PublishedInstaller"
```

- [ ] **Step 4: 记录正式安装版打包命令**

Add this command beside the existing portable package command in `CONTRIBUTING.md`:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\package-installer.ps1
```

- [ ] **Step 5: 验证配置和脚本语法**

Run:

```powershell
$config = Get-Content -Raw src-tauri\tauri.conf.json | ConvertFrom-Json
if ($config.bundle.windows.nsis.installerIcon -ne "icons/icon.ico") {
    throw "NSIS installer icon is not configured."
}
if ($config.bundle.windows.nsis.uninstallerIcon -ne "icons/icon.ico") {
    throw "NSIS uninstaller icon is not configured."
}
[void][scriptblock]::Create((Get-Content -Raw scripts\package-installer.ps1))
```

Expected: PASS with exit code 0.

- [ ] **Step 6: 运行源码验证**

Run:

```powershell
npm test
npm run build
cargo fmt --manifest-path src-tauri\Cargo.toml --all -- --check
cargo test --manifest-path src-tauri\Cargo.toml
```

Expected: 前端 63 项测试通过，前端构建成功；Rust 54 项测试通过，格式检查成功。

- [ ] **Step 7: 生成公开安装包**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\package-installer.ps1
```

Expected: 成功生成 `release/Wall-1.0.0-x64-setup.exe`。

- [ ] **Step 8: 验证文件名、内容与安装器图标**

Run:

```powershell
$installer = "release\Wall-1.0.0-x64-setup.exe"
if ((Split-Path -Leaf $installer).Contains("_")) {
    throw "Published installer filename contains an underscore."
}
& 7z t $installer | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "Installer integrity check failed."
}
$listing = (& 7z l $installer) -join "`n"
foreach ($name in @("wall.exe", "mpv.exe", "d3dcompiler_43.dll")) {
    if ($listing -notmatch [regex]::Escape($name)) {
        throw "Installer is missing $name."
    }
}

Add-Type -AssemblyName System.Drawing
$sourceIcon = [Drawing.Icon]::new((Resolve-Path "src-tauri\icons\icon.ico").Path, 32, 32)
$installerIcon = [Drawing.Icon]::ExtractAssociatedIcon((Resolve-Path $installer).Path)
try {
    $sourceBitmap = $sourceIcon.ToBitmap()
    $installerBitmap = $installerIcon.ToBitmap()
    try {
        for ($x = 0; $x -lt 32; $x++) {
            for ($y = 0; $y -lt 32; $y++) {
                if ($sourceBitmap.GetPixel($x, $y) -ne $installerBitmap.GetPixel($x, $y)) {
                    throw "Installer icon does not match icons/icon.ico."
                }
            }
        }
    } finally {
        $sourceBitmap.Dispose()
        $installerBitmap.Dispose()
    }
} finally {
    $sourceIcon.Dispose()
    $installerIcon.Dispose()
}
```

Expected: PASS with exit code 0.

- [ ] **Step 9: 检查差异并提交**

Run:

```powershell
$cargoPath = (Resolve-Path "src-tauri\Cargo.toml").Path
$cargoContent = [IO.File]::ReadAllText($cargoPath).Replace("`r`n", "`n").Replace("`n", "`r`n")
[IO.File]::WriteAllText($cargoPath, $cargoContent, [Text.UTF8Encoding]::new($false))
git diff --check
git status --short
git add src-tauri/tauri.conf.json scripts/package-installer.ps1 CONTRIBUTING.md
git commit -m "fix(release): 统一安装包图标与文件名"
```

Expected: 只提交配置、安装版脚本和发布文档，生成的安装包继续由 `.gitignore` 排除。

- [ ] **Step 10: 输出哈希与工作区状态**

Run:

```powershell
Get-FileHash -Algorithm SHA256 release\Wall-1.0.0-x64-setup.exe
git status --short --branch
```

Expected: 安装包有 SHA-256，工作区无未提交文件。
