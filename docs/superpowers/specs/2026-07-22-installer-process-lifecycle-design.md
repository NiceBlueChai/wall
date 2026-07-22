<!-- Wall 二次安装进程生命周期与中文安装器设计规格。 -->

# 二次安装进程生命周期与中文安装器设计

## 目标

Wall 正在播放壁纸时再次运行安装程序，安装器应能关闭 Wall 及其所属 mpv，覆盖安装文件并继续安装；
安装器和卸载器仅使用简体中文，不显示语言选择框。

## 已确认行为

- Wall 启动的 mpv 必须属于 Wall 的进程生命周期；Wall 正常退出、被安装器结束或异常退出时，mpv 都应退出。
- 从尚未具备该生命周期约束的旧版本升级时，安装器仅清理可执行路径等于 `$INSTDIR\mpv.exe` 的残留进程。
- 不按进程名结束系统中的全部 `mpv.exe`，避免影响用户自行运行的 mpv。
- 安装器仅包含 NSIS 的 `SimpChinese` 语言，不显示语言选择框。
- 不改变壁纸播放、媒体文件、用户配置和离线产品边界。

## 方案

### 运行时根治

在现有 `MpvPlayer` 启动子进程的共享路径中创建 Windows Job Object，启用
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，并把新启动的 mpv 分配给该 Job Object。Job 句柄由播放器实例持有；
Wall 进程终止后，Windows 关闭句柄并终止对应 mpv。现有正常退出时的 IPC `quit` 和 `Child::kill` 逻辑继续保留。

若创建 Job Object、设置限制或分配子进程失败，本次播放启动失败并回收已经启动的 mpv，不能悄悄退化为可能残留的播放。

### 旧版本升级兜底

通过 Tauri NSIS 的 <code>NSIS_HOOK_PREINSTALL</code> 在复制文件前检查进程。只结束可执行路径与
`$INSTDIR\mpv.exe` 相同的进程，并等待文件解除占用；找不到匹配进程时直接继续。

该钩子用于第一次从旧版本升级。安装新版本后，Job Object 是后续退出和升级的主要保障。

### 中文安装器

在 Tauri NSIS 配置中设置 `languages: ["SimpChinese"]`，保持
`displayLanguageSelector: false`。使用 Tauri 和 NSIS 自带简体中文文本，不维护自定义语言包。

## 备选方案

1. 仅在安装器中结束 mpv：能处理安装，却不能处理 Wall 崩溃或被任务管理器结束后的残留，拒绝。
2. 仅增加 Tauri 退出事件清理：安装器或系统强制终止进程时不保证执行清理，拒绝。
3. Job Object 加安装路径限定的升级钩子：同时覆盖新版本生命周期和旧版本迁移，采用。

## 所有者与证明

| 所有者 | 分类 | 修改 | 证明 |
| --- | --- | --- | --- |
| Rust 播放器进程管理 | required enforcement | Job Object 约束全部受管 mpv | Rust 单元测试与 Windows 进程实测 |
| NSIS 安装边界 | required enforcement | 清理旧版本安装路径内的残留 mpv | 二次安装实测 |
| Tauri 安装器本地化 | required presentation | 仅打包简体中文 | 安装器界面实测 |
| 前端与持久化 | no impact | 无修改 | 现有测试与构建 |
| 用户源媒体 | no impact | 不读写、不移动、不删除 | 代码审查 |

## 验证

1. 先增加可运行的最小回归测试，证明 Job Object 配置失败时会回收子进程，且成功路径持有 Job 句柄。
2. 运行 `cargo fmt --manifest-path src-tauri\Cargo.toml --all -- --check`。
3. 运行 `cargo test --manifest-path src-tauri\Cargo.toml`。
4. 运行 <code>npm test</code> 和 <code>npm run build</code>。
5. 打包 NSIS 安装程序，确认界面为简体中文。
6. Windows 实机启动 Wall 并播放壁纸，再运行安装程序；确认 Wall 与其 mpv 退出、安装继续且壁纸可恢复。

## 非目标

- 不增加新的第三方依赖。
- 不制作多语言选择界面或自定义 NSIS 模板。
- 不结束安装目录之外的 mpv。
- 不改变安装目录、版本号或发布文件命名规则。
