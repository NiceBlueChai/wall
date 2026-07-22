<!-- Wall 二次安装进程生命周期与中文安装器的实现边界和验证记录。 -->

# 二次安装进程生命周期与中文安装器报告

日期：2026-07-22

## 用户契约

用户可以：

- 在 Wall 正在运行并播放视频壁纸时再次运行安装程序。
- 由安装程序关闭旧版本遗留、且属于同一安装目录的 mpv 进程，再安全覆盖程序文件。
- 使用简体中文完成安装，不需要操作语言选择器。
- 在 Wall 被强制终止时，由 Windows 同步结束 Wall 创建的 mpv 子进程。

用户不能：

- 让安装程序按进程名误杀其他目录中的同名 mpv 进程。
- 因强制关闭 Wall 而长期遗留仍占用安装目录文件的 mpv 进程。
- 通过安装或进程清理删除、移动或修改任何用户壁纸源文件。

## 设计与实现边界

设计说明：[installer-process-lifecycle-design.md](../superpowers/specs/2026-07-22-installer-process-lifecycle-design.md)

实施计划：[installer-process-lifecycle.md](../superpowers/plans/2026-07-22-installer-process-lifecycle.md)

| 契约面 | 所有者 | 实现 |
| --- | --- | --- |
| 播放器生命周期 | Rust 播放器层 | 使用带 `KILL_ON_JOB_CLOSE` 的 Windows Job Object 绑定 Wall 创建的 mpv。 |
| 旧版本迁移 | NSIS 安装钩子 | 安装前按完整可执行文件路径清理安装目录内残留的 mpv，并等待退出。 |
| 安装器语言 | Tauri NSIS 配置 | 只打包简体中文语言表，关闭语言选择器。 |
| 壁纸数据 | 既有存储与媒体层 | 本轮不修改库记录、分类、设置或用户源媒体。 |
| 前端界面 | 既有 Vue 界面 | 本轮不修改应用内界面或播放状态机。 |

该问题横跨 Rust 进程所有权和 NSIS 升级边界，但修复范围小且强耦合，因此在同一工作流中完成；没有新增依赖或第二套播放状态。

## 根因与修复

旧实现仅在 Wall 正常退出时主动停止 mpv。安装器强制终止 Wall 会绕过 Rust 析构流程，使 mpv 成为孤儿进程并继续锁定安装目录中的 `mpv.exe`，从而导致二次安装覆盖失败。

新实现让 Windows 内核管理父子生命周期：Wall 持有 Job Object，进程退出后句柄关闭，系统终止其中的 mpv。安装器同时提供旧版本兼容路径，仅清理可执行文件路径与目标安装目录完全一致的 mpv；清理失败时中止覆盖并显示中文错误，不忽略被锁文件。

## 自动化证明

| 验证 | 结果 |
| --- | --- |
| `cargo fmt --manifest-path src-tauri\Cargo.toml --all -- --check` | 通过。 |
| `cargo test --manifest-path src-tauri\Cargo.toml` | 55 项通过，包含 Job Object 强制退出集成测试。 |
| `npm test` | 8 个测试文件、63 项测试通过。 |
| `npm run build` | Vue / TypeScript 检查和 Vite 生产构建通过。 |
| `scripts\verify-installer-lifecycle.ps1` | 通过；目标路径进程退出，其他目录同名进程保留。 |
| `7z t release\Wall-1.0.0-x64-setup.exe` | 通过；NSIS 3 Unicode 安装包完整。 |

安装器生命周期脚本使用真实 Windows 进程验证路径筛选，但用复制并改名的系统程序模拟 mpv，不访问用户媒体。Job Object 测试直接调用真实 Win32 API，不使用进程替身。

## Windows 实机验证

- 使用修复后的中文安装器覆盖旧版本遗留的真实 mpv，成功越过原先的 `mpv.exe` 写入失败点并完成安装。
- 安装器标题、进度、详情和操作文案显示为简体中文。
- 强制结束已安装的 Wall 后，其创建的 mpv 同步退出；重新启动 Wall 后视频壁纸恢复。
- 用户已在原始二次安装场景中确认修复有效，并明确无需再次重复安装验证。

## 发布产物

- 文件：`release\Wall-1.0.0-x64-setup.exe`
- 大小：37,095,900 字节
- SHA-256：`149D42B740CF4EB9A0476A5D5566DBACC73C219F2EBF0AEE4BA58288E46E81E7`
- 数字签名：未签名。

## 已知边界

- 安装前清理依赖 Windows PowerShell 5.1 和 CIM，符合项目的 Windows 10/11 x64 产品边界。
- 当前安装包没有代码签名；发布时 Windows 可能显示未知发布者提示。
