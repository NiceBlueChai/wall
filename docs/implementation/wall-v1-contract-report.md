<!-- Wall v1 产品契约、实现边界和验收证据。 -->
# Wall v1 实现报告

## 已实现契约

- Windows 10/11 x64、完全离线、本地视频与图片、MIT 源码和便携 ZIP。
- Vue 3 四页设置、壁纸库、详情、搜索筛选、文件丢失提示和统一错误展示。
- Rust 状态机、三文件原子持久化、暂停原因集合、托盘、单实例和开机启动开关。
- WorkerW 桌面嵌入、mpv 命名管道控制、Cover/Contain/Stretch、音量、静音和 60 FPS。
- 全屏、电池、显示器休眠自动暂停；手动暂停不会被自动条件误恢复。
- 本地资源协议仅动态放行已导入文件；运行时没有商城、账号、遥测、更新检查或网络请求。

## 所有者与边界

| 契约 | 所有者 | 证明 |
| --- | --- | --- |
| 界面与设置交互 | `src/` | Vitest 6 项 |
| 状态、媒体和持久化 | `src-tauri/src/core.rs`、`media.rs`、`storage.rs` | Cargo 16 项 |
| 桌面播放与 IPC | `player.rs`、`commands.rs` | 参数测试与便携进程命令行 |
| 托盘和系统条件 | `tray.rs`、`monitor.rs` | 共享 `WallCore` 状态 |
| 离线发行 | `scripts/prepare-mpv.ps1`、`package-portable.ps1` | SHA-256 与 ZIP 内容检查 |

## 验收证据

- `npm run test`：5 个测试文件、6 项通过。
- `cargo test --manifest-path src-tauri/Cargo.toml`：16 项通过。
- `npm run build`：Vue 类型检查与 Vite 生产构建通过。
- 本地 Tauri CLI `build --no-bundle`：生成 `src-tauri/target/release/wall.exe`。
- 便携版 PE Subsystem 为 2（Windows GUI），启动时不创建控制台窗口。
- 便携目录中的 mpv 成功播放
  `C:\Users\m1506\Downloads\73844-549547529_medium.mp4`，命令行包含 WorkerW、IPC、Cover、
  60 FPS、硬件解码和静音参数。

## 已知边界

- 当前机器完成了 Windows 实机验证；仍需在独立 Windows 10 和 Windows 11 干净系统上做发布前矩阵验证。
- 媒体尺寸、时长和离线缩略图探测尚未接入；当前列表和详情直接使用 WebView 本地媒体预览。
- mpv 子进程崩溃会在下一次控制操作时显示错误，尚未增加主动进程退出监听。
- 项目主页未确定，关于页按钮按计划保持禁用。

本次实现按单工作流完成；项目规则禁止在未明确授权时启用并行子代理，因此没有拆分代理通道。
