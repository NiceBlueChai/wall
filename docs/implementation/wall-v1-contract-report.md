<!-- Wall v1 产品契约、实现边界和验收证据。 -->

# Wall v1 实现报告

## 已实现契约

- Windows 10/11 x64、完全离线、本地视频与图片、MIT 源码和便携 ZIP。
- Vue 3 四页设置、壁纸库、详情、搜索筛选、文件丢失提示和统一错误展示。
- 关于页显示作者 `NiceBlueChai` 和联系邮箱 `bluechai@qq.com`。
- 关于页仅在用户主动点击后，将 `https://github.com/NiceBlueChai/wall` 交给系统默认浏览器。
- Rust 状态机、三文件原子持久化、暂停原因集合、托盘、单实例和开机启动开关。
- WorkerW 桌面嵌入、mpv 命名管道控制、Cover/Contain/Stretch、音量、静音和 60 FPS。
- 全屏、电池、显示器休眠自动暂停；手动暂停不会被自动条件误恢复。
- 本地资源协议仅动态放行已导入文件；Wall 进程没有商城、账号、遥测、更新检查或 HTTP 请求。

## 所有者与边界

| 契约               | 所有者                                            | 证明                     |
| ------------------ | ------------------------------------------------- | ------------------------ |
| 界面与设置交互     | `src/`                                            | Vitest 7 个文件、9 项    |
| 状态、媒体和持久化 | `src-tauri/src/core.rs`、`media.rs`、`storage.rs` | Cargo 16 项              |
| 桌面播放与 IPC     | `player.rs`、`commands.rs`                        | 参数测试与便携进程命令行 |
| 项目主页系统边界   | `SettingsView.vue`、`commands.rs`                 | 组件测试与固定 URL 测试  |
| 托盘和系统条件     | `tray.rs`、`monitor.rs`                           | 共享 `WallCore` 状态     |
| 离线发行           | `scripts/prepare-mpv.ps1`、`package-portable.ps1` | SHA-256 与 ZIP 内容检查  |

项目主页改动范围小且写入点集中，串行修改可避免 Vue、Rust 和本文档之间的共享文件冲突，未拆分并行任务。

## 项目主页行为契约

| 层/模块    | 原行为         | 新行为             | 用户可以         | 用户不能         | 证明                    |
| ---------- | -------------- | ------------------ | ---------------- | ---------------- | ----------------------- |
| Vue 关于页 | 按钮禁用       | 按钮可点击         | 主动请求打开主页 | 在应用内浏览网页 | 渲染组件与点击测试      |
| Rust 命令  | 返回未配置错误 | 调用系统默认浏览器 | 打开固定官方仓库 | 传入任意外部 URL | 常量测试与 release 编译 |
| 文档       | 地址未确定     | 中英文说明实际地址 | 核对仓库目的地   | 误解为隐式联网   | README 与设计规范       |

组件测试使用 Tauri API 替身；Rust release 构建覆盖真实 opener 集成。自动化没有实际启动用户浏览器，最终视觉
证据为 N/A，替代证据是按钮渲染/点击测试、固定 URL 单元测试和 Windows release 编译。

## 验收证据

- `npm run test`：7 个测试文件、9 项通过。
- `cargo test --manifest-path src-tauri/Cargo.toml`：16 项通过。
- `npm run build`：Vue 类型检查与 Vite 生产构建通过。
- 本地 Tauri CLI `build --no-bundle`：生成 `src-tauri/target/release/wall.exe`。
- 便携版 PE Subsystem 为 2（Windows GUI），启动时不创建控制台窗口。
- 便携目录中的 mpv 已使用本地 MP4 实机验证；命令行包含 WorkerW、IPC、Cover、60 FPS、硬件解码和
  静音参数。
- 发布包中的三个测试视频由项目脚本使用 FFmpeg 测试源生成，不包含来源不明的第三方媒体。

## 开源发布契约

- 源码、项目图标和生成的示例媒体可以按根目录 MIT License 再分发。
- 便携包必须同时包含中英文 README、`LICENSE`、`THIRD_PARTY_NOTICES.md` 和示例媒体说明。
- 发布脚本只能打包项目生成或具有明确再分发许可的媒体，不能读取开发者下载目录。
- FFmpeg 只在构建时生成测试图案，不进入便携包；mpv 运行文件按其独立许可声明分发。
- CI 负责前端测试、前端构建、Rust 格式和 Rust 测试；Windows 原生便携构建仍由发布脚本完成。

## 已知边界

- 当前机器完成了 Windows 实机验证；仍需在独立 Windows 10 和 Windows 11 干净系统上做发布前矩阵验证。
- 媒体尺寸、时长和离线缩略图探测尚未接入；当前列表和详情直接使用 WebView 本地媒体预览。
- mpv 子进程崩溃会在下一次控制操作时显示错误，尚未增加主动进程退出监听。
- 系统默认浏览器的实际启动仍需在发布前做一次人工冒烟验证；自动化只验证按钮、命令地址和原生集成编译。
