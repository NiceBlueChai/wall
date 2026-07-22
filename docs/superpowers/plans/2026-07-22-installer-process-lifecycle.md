<!-- Wall 二次安装进程生命周期与中文安装器实现计划。 -->

# 二次安装进程生命周期与中文安装器 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wall 被安装器或系统结束时自动终止所属 mpv，并让旧版本二次安装顺利完成且全程显示简体中文。

**Architecture:** 在现有 mpv 统一启动路径中使用 Windows Job Object 绑定父子进程生命周期；NSIS 预安装钩子仅按
`$INSTDIR\mpv.exe` 的完整路径清理旧版本残留。Tauri 配置只打包 `SimpChinese`，不维护自定义语言包。

**Tech Stack:** Rust 2024、Tauri 2、windows 0.61、Windows Job Objects、NSIS、Windows PowerShell 5.1。

## Global Constraints

- 仅支持 Windows 10/11 x64、本地媒体和完全离线运行。
- 不新增第三方依赖，只扩展现有 `windows` crate 的功能开关。
- 不按进程名结束安装目录之外的 mpv。
- 不读取、移动、修改或删除用户源媒体。
- 安装器仅包含简体中文，不显示语言选择框。
- 保持现有安装目录、版本号和 `Wall-<version>-x64-setup.exe` 发布命名规则。
- 所有源码使用 UTF-8、CRLF、4 空格缩进，源码单行不超过 120 字符。

---

### Task 1: 用 Windows Job Object 绑定 mpv 生命周期

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/player.rs`
- Test: `src-tauri/src/player.rs`

**Interfaces:**
- Consumes: 现有 `MpvPlayer::play_region` 的 `std::process::Child`。
- Produces: `assign_child_to_kill_job(child: &Child) -> Result<OwnedHandle, PlayerError>`；
  `MpvPlayer` 在播放期间持有 Job 句柄。

- [ ] **Step 1: 写入会失败的 Windows 进程生命周期测试**

在 `player.rs` 的测试模块导入 `assign_child_to_kill_job`、`terminate_child`，并加入：

```rust
#[cfg(windows)]
#[test]
fn dropping_job_terminates_assigned_child() {
    use std::os::windows::process::CommandExt as _;
    use std::process::Command;

    let mut command = Command::new("powershell.exe");
    command
        .args(["-NoLogo", "-NoProfile", "-Command", "Start-Sleep -Seconds 30"])
        .creation_flags(0x08000000);
    let mut child = command.spawn().expect("测试子进程应能启动");
    let job = assign_child_to_kill_job(&child).expect("子进程应能加入 Job Object");

    drop(job);
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if child.try_wait().expect("应能读取测试子进程状态").is_some() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }

    terminate_child(&mut child);
    panic!("Job Object 关闭后测试子进程仍在运行");
}
```

同时在测试模块加入：

```rust
#[cfg(windows)]
use super::{assign_child_to_kill_job, terminate_child};
#[cfg(windows)]
use std::{
    thread,
    time::{Duration, Instant},
};
```

- [ ] **Step 2: 运行测试并确认因缺少 Job Object 实现而失败**

Run:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml dropping_job_terminates_assigned_child
```

Expected: FAIL，提示找不到 `assign_child_to_kill_job`。

- [ ] **Step 3: 启用现有 windows crate 所需功能**

在 `src-tauri/Cargo.toml` 的 Windows features 中加入：

```toml
"Win32_Security",
"Win32_System_JobObjects",
```

- [ ] **Step 4: 实现最小 Job Object 绑定**

在 `player.rs` 中按 Windows 条件导入 `std::os::windows::io::OwnedHandle`，为 `MpvPlayer` 增加：

```rust
#[cfg(windows)]
job: Option<OwnedHandle>,
```

在 `terminate_child` 附近加入：

```rust
#[cfg(windows)]
fn assign_child_to_kill_job(child: &Child) -> Result<OwnedHandle, PlayerError> {
    use core::ffi::c_void;
    use std::mem::size_of;
    use std::os::windows::io::{AsRawHandle as _, FromRawHandle as _};
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };
    use windows::core::PCWSTR;

    let raw_job = unsafe { CreateJobObjectW(None, PCWSTR::null()) }
        .map_err(|problem| std::io::Error::other(problem.to_string()))?;
    let job = unsafe { OwnedHandle::from_raw_handle(raw_job.0) };
    let job_handle = HANDLE(job.as_raw_handle());
    let mut information = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    information.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

    unsafe {
        SetInformationJobObject(
            job_handle,
            JobObjectExtendedLimitInformation,
            &information as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION as *const c_void,
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
        .map_err(|problem| std::io::Error::other(problem.to_string()))?;
        AssignProcessToJobObject(job_handle, HANDLE(child.as_raw_handle()))
            .map_err(|problem| std::io::Error::other(problem.to_string()))?;
    }
    Ok(job)
}
```

在 `play_region` 的 `command.spawn()?` 之后立即加入：

```rust
let mut child = command.spawn()?;
#[cfg(windows)]
let job = match assign_child_to_kill_job(&child) {
    Ok(job) => job,
    Err(problem) => {
        terminate_child(&mut child);
        return Err(problem);
    }
};
```

成功完成窗口嵌入后，在现有 `self.child = Some(child)` 旁加入：

```rust
#[cfg(windows)]
{
    self.job = Some(job);
}
```

在 `stop` 清空 `child` 后加入：

```rust
#[cfg(windows)]
{
    self.job = None;
}
```

- [ ] **Step 5: 运行定向测试并确认通过**

Run:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml dropping_job_terminates_assigned_child
```

Expected: PASS；关闭 Job 句柄后，测试 PowerShell 子进程在 3 秒内退出。

- [ ] **Step 6: 运行播放器相关测试**

Run:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml player::tests
```

Expected: PASS，无残留 `powershell.exe` 测试进程。

- [ ] **Step 7: 提交运行时修复**

```powershell
git add src-tauri/Cargo.toml src-tauri/src/player.rs
git commit -m "fix(player): 绑定 mpv 与应用进程生命周期"
```

### Task 2: 清理旧版本残留并启用中文 NSIS

**Files:**
- Create: `scripts/verify-installer-lifecycle.ps1`
- Create: `src-tauri/windows/cleanup-mpv.ps1`
- Create: `src-tauri/windows/installer-hooks.nsh`
- Modify: `src-tauri/tauri.conf.json`

**Interfaces:**
- Consumes: NSIS 的 `$INSTDIR`、`NSIS_HOOK_PREINSTALL` 和 Windows PowerShell 5.1。
- Produces: `cleanup-mpv.ps1 -TargetPath <absolute-path>`，只终止可执行路径完全匹配的 mpv。

- [ ] **Step 1: 写入会失败的安装器契约验证脚本**

创建 `scripts/verify-installer-lifecycle.ps1`：

```powershell
<# Verifies the Wall installer language and exact-path mpv cleanup contract. #>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$ConfigPath = Join-Path $ProjectRoot "src-tauri\tauri.conf.json"
$CleanupScript = Join-Path $ProjectRoot "src-tauri\windows\cleanup-mpv.ps1"
$HookScript = Join-Path $ProjectRoot "src-tauri\windows\installer-hooks.nsh"
$Config = Get-Content -LiteralPath $ConfigPath -Raw -Encoding UTF8 | ConvertFrom-Json
$Languages = @($Config.bundle.windows.nsis.languages)

if ($Languages.Count -ne 1 -or $Languages[0] -ne "SimpChinese") {
    throw "NSIS installer languages must contain only SimpChinese."
}
if ($Config.bundle.windows.nsis.displayLanguageSelector -ne $false) {
    throw "NSIS installer language selector must be disabled."
}
if ($Config.bundle.windows.nsis.installerHooks -ne "windows/installer-hooks.nsh") {
    throw "NSIS installer hook path is not configured."
}
foreach ($file in @($CleanupScript, $HookScript)) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        throw "Required installer lifecycle file is missing: $file"
    }
}
[void][scriptblock]::Create((Get-Content -LiteralPath $CleanupScript -Raw -Encoding UTF8))

$TempName = "wall-installer-lifecycle-" + [guid]::NewGuid()
$TempRoot = Join-Path ([IO.Path]::GetTempPath()) $TempName
$TargetDirectory = Join-Path $TempRoot "target"
$OtherDirectory = Join-Path $TempRoot "other"
$TargetMpv = Join-Path $TargetDirectory "mpv.exe"
$OtherMpv = Join-Path $OtherDirectory "mpv.exe"
$TargetProcess = $null
$OtherProcess = $null

try {
    New-Item -ItemType Directory -Force -Path $TargetDirectory, $OtherDirectory | Out-Null
    Copy-Item -LiteralPath $env:ComSpec -Destination $TargetMpv
    Copy-Item -LiteralPath $env:ComSpec -Destination $OtherMpv
    $Arguments = @("/d", "/c", "ping 127.0.0.1 -n 30 > nul")
    $StartArguments = @{
        ArgumentList = $Arguments
        PassThru = $true
        WindowStyle = "Hidden"
    }
    $TargetProcess = Start-Process -FilePath $TargetMpv @StartArguments
    $OtherProcess = Start-Process -FilePath $OtherMpv @StartArguments
    Start-Sleep -Milliseconds 250

    & $CleanupScript -TargetPath $TargetMpv
    if (-not $TargetProcess.WaitForExit(5000)) {
        throw "The target mpv process was not stopped."
    }
    $OtherProcess.Refresh()
    if ($OtherProcess.HasExited) {
        throw "An mpv process outside the target path was stopped."
    }
} finally {
    foreach ($process in @($TargetProcess, $OtherProcess)) {
        if ($null -ne $process) {
            $process.Refresh()
            if (-not $process.HasExited) {
                Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
            }
            $process.Dispose()
        }
    }
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}

Write-Host "Installer lifecycle contract verified."
```

- [ ] **Step 2: 运行验证脚本并确认因缺少中文与清理配置而失败**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\verify-installer-lifecycle.ps1
```

Expected: FAIL with `NSIS installer languages must contain only SimpChinese.`

- [ ] **Step 3: 实现按完整路径清理 mpv 的脚本**

创建 `src-tauri/windows/cleanup-mpv.ps1`：

```powershell
<# Stops only mpv processes whose executable path matches Wall's installed mpv. #>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$TargetPath
)

$ErrorActionPreference = "Stop"
$ResolvedTarget = [IO.Path]::GetFullPath($TargetPath)
$Processes = @(Get-CimInstance Win32_Process -Filter "Name = 'mpv.exe'" | Where-Object {
    $_.ExecutablePath -and [IO.Path]::GetFullPath($_.ExecutablePath).Equals(
        $ResolvedTarget,
        [StringComparison]::OrdinalIgnoreCase
    )
})

foreach ($process in $Processes) {
    Stop-Process -Id $process.ProcessId -Force -ErrorAction Stop
}

$Deadline = [DateTime]::UtcNow.AddSeconds(5)
foreach ($process in $Processes) {
    while (Get-Process -Id $process.ProcessId -ErrorAction SilentlyContinue) {
        if ([DateTime]::UtcNow -ge $Deadline) {
            throw "Timed out waiting for mpv process $($process.ProcessId) to stop."
        }
        Start-Sleep -Milliseconds 50
    }
}
```

- [ ] **Step 4: 接入最小 NSIS 预安装钩子**

创建 `src-tauri/windows/installer-hooks.nsh`：

```nsh
; Stops an orphaned Wall mpv from versions that predate Job Object ownership.
!define WALL_POWERSHELL "$SYSDIR\WindowsPowerShell\v1.0\powershell.exe"
!define WALL_CLEANUP_SCRIPT "$PLUGINSDIR\wall-cleanup-mpv.ps1"

!macro NSIS_HOOK_PREINSTALL
    InitPluginsDir
    File /oname=${WALL_CLEANUP_SCRIPT} "${__FILEDIR__}\cleanup-mpv.ps1"
    nsExec::ExecToStack \
        '"${WALL_POWERSHELL}" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass \
        -File "${WALL_CLEANUP_SCRIPT}" -TargetPath "$INSTDIR\mpv.exe"'
    Pop $0
    Pop $1
    StrCmp $0 "0" wall_cleanup_mpv_done
    MessageBox MB_ICONSTOP|MB_OK "无法结束旧版 Wall 播放器：$1"
    Abort
wall_cleanup_mpv_done:
!macroend
```

- [ ] **Step 5: 配置简体中文与安装器钩子**

把 `src-tauri/tauri.conf.json` 的 `bundle.windows.nsis` 更新为：

```json
"nsis": {
    "installerIcon": "icons/icon.ico",
    "uninstallerIcon": "icons/icon.ico",
    "installerHooks": "windows/installer-hooks.nsh",
    "languages": ["SimpChinese"],
    "displayLanguageSelector": false
}
```

- [ ] **Step 6: 运行安装器契约验证并确认通过**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\verify-installer-lifecycle.ps1
```

Expected: PASS with `Installer lifecycle contract verified.`；目标路径的测试 mpv 退出，另一目录的同名进程继续运行。

- [ ] **Step 7: 提交安装器修复**

```powershell
git add scripts/verify-installer-lifecycle.ps1 src-tauri/windows/cleanup-mpv.ps1
git add src-tauri/windows/installer-hooks.nsh src-tauri/tauri.conf.json
git commit -m "fix(installer): 清理残留播放器并启用中文"
```

### Task 3: 完整验证、打包与交付记录

**Files:**
- Create: `docs/implementation/installer-process-lifecycle-report.md`
- Verify: `release/Wall-1.0.0-x64-setup.exe`

**Interfaces:**
- Consumes: Task 1 的 Job Object、Task 2 的 NSIS hook 和中文配置。
- Produces: 可发布安装包与可追溯验证报告。

- [ ] **Step 1: 运行仓库要求的自动化检查**

Run:

```powershell
cargo fmt --manifest-path src-tauri\Cargo.toml --all -- --check
cargo test --manifest-path src-tauri\Cargo.toml
npm test
npm run build
powershell -ExecutionPolicy Bypass -File scripts\verify-installer-lifecycle.ps1
```

Expected: 全部 PASS，无警告导致的失败，也无测试子进程残留。

- [ ] **Step 2: 构建安装版并验证包结构**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\package-installer.ps1
7z t release\Wall-1.0.0-x64-setup.exe
```

Expected: 两个命令退出码均为 0，发布文件名不含下划线。

- [ ] **Step 3: 执行 Windows 二次安装实测**

1. 启动当前安装版 Wall 并播放任意本地视频或图片壁纸。
2. 运行新生成的 `release\Wall-1.0.0-x64-setup.exe`。
3. 确认安装界面和提示均为简体中文，没有语言选择框。
4. 在“Wall 正在运行”的提示中继续安装。
5. 确认旧 `Wall.exe` 和路径为 `$INSTDIR\mpv.exe` 的进程退出，安装器没有“Error opening file for writing”。
6. 安装完成后启动 Wall，确认壁纸可再次播放。
7. 另启安装目录之外的 `mpv.exe` 时重复测试，确认该进程不会被安装器结束。

Expected: 所有七项均通过，用户源媒体和配置保持不变。

- [ ] **Step 4: 写入实现报告**

创建 `docs/implementation/installer-process-lifecycle-report.md`：

```markdown
<!-- Wall 二次安装进程生命周期与中文安装器实现及验证报告。 -->

# 二次安装进程生命周期与中文安装器报告

## 产品契约

- Wall 退出或被结束时，其受管 mpv 随之退出。
- 旧版本升级只清理安装目录中的 mpv，不影响其他 mpv。
- NSIS 安装器和卸载器仅显示简体中文。

## 所有者结果

| 所有者 | 分类 | 结果 |
| --- | --- | --- |
| Rust 播放器进程管理 | required enforcement | implemented |
| NSIS 安装边界 | required enforcement | implemented |
| Tauri 安装器本地化 | required presentation | implemented |
| 前端与持久化 | no impact | no impact |
| 用户源媒体 | no impact | no impact |

## 验证结果

| 检查 | 结果 |
| --- | --- |
| Rust 格式检查与测试 | PASS |
| 前端测试与构建 | PASS |
| 安装器路径隔离脚本 | PASS |
| NSIS 构建与压缩包检查 | PASS |
| Windows 播放中二次安装 | PASS |
| 安装目录外 mpv 保留 | PASS |
| 用户源媒体和配置保持不变 | PASS |

## 边界与剩余风险

- 实机边界使用 Windows 10/11 x64、Windows PowerShell 5.1 和真实 NSIS 安装包验证。
- 不支持 Windows 10 以前的系统，不提供多语言选择器。
```

- [ ] **Step 5: 检查差异并提交报告**

Run:

```powershell
git diff --check
git status --short
git add docs/implementation/installer-process-lifecycle-report.md
git commit -m "docs(installer): 记录二次安装验证结果"
```

Expected: `git diff --check` 无输出；提交只包含验证报告。

- [ ] **Step 6: 最终提交门禁**

Run:

```powershell
git status --short
git log -4 --oneline
```

Expected: 工作区干净，最近提交依次包含运行时修复、安装器修复和验证报告。
