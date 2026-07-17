<!-- Wall 的中文用户指南、开发构建和开源协作入口。 -->

<p align="right">
    <a href="README.md">English</a> | 简体中文
</p>

<p align="center">
    <img src="public/wall-app-icon.png" width="96" alt="Wall 应用图标">
</p>

# Wall

Wall 是面向 Windows 10/11 x64 的免费开源动态壁纸工具。它只读取用户选择的本地视频和图片，
不提供账号、商城、云同步、遥测或自动更新。

<p align="center">
    <img src="docs/images/wall-library.png" width="960" alt="Wall 壁纸库界面">
</p>

## 功能

- 本地视频与图片壁纸，支持用户分类和批量管理。
- Cover、Contain、Stretch、常用画幅、三级抗锯齿，以及源/24/30/60 FPS。
- 壁纸级覆盖与全局继承，支持 mpv 硬件解码、循环播放、静音和音量控制。
- 多显示器独立、复制、铺展播放，以及按显示目标操作的托盘菜单。
- 全屏或最大化应用、电池供电和显示器休眠时自动暂停。
- 系统托盘、开机启动、关闭到托盘、显示器热插拔保留和恢复上次壁纸。
- 完全离线运行；只有用户主动点击[项目主页](https://github.com/NiceBlueChai/wall)时才会交给系统浏览器。

## 下载与使用

1. 从 GitHub Releases 下载 `Wall-v1.0.0-windows-x64-portable.zip`。
2. 解压整个目录，不要只复制 `Wall.exe`。
3. 运行 `Wall.exe`，点击“导入壁纸”选择本地视频或图片。
4. 便携包的 `Sample Wallpapers` 目录包含三个由项目脚本生成的测试视频。

便携版不捆绑 WebView2 固定运行时。Windows 10/11 通常已安装；缺失时请安装 Microsoft Edge
WebView2 Runtime。应用和壁纸播放本身不需要联网。

## 支持范围

- 操作系统：Windows 10/11 x64。
- 媒体：本地 MP4、WebM、MKV、MOV、AVI、JPG、JPEG、PNG、WebP、BMP 和 GIF。
- 显示器：支持主屏和副屏的独立、复制、铺展布局。
- 不支持：在线商城、网页壁纸、互动壁纸、播放列表和定时轮换。

## 隐私与数据

- Wall 不上传媒体，不修改用户原文件，也不发送遥测。
- 壁纸库、设置和播放会话保存在当前用户的应用数据目录。
- 运行日志保存在当前用户的应用配置目录，可从设置页打开日志目录。
- 运行时不检查更新，不发起 HTTP 请求。

## 开发

前置环境：Node.js 22、Rust stable、Microsoft C++ Build Tools、WebView2 Runtime 和 7-Zip。

```powershell
npm install
powershell -ExecutionPolicy Bypass -File scripts\prepare-mpv.ps1
npm run tauri dev
```

## 测试与构建

```powershell
npm run test
cargo test --manifest-path src-tauri\Cargo.toml
npm run build
```

生成带有三个自有测试视频的便携包还需要 `ffmpeg` 可从命令行调用：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\package-portable.ps1
```

无需鼠标的原生冒烟测试会启动便携版、验证 WorkerW 层级和 GUI 子系统，并在结束后按 SHA-256 恢复原应用数据：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\verify-portable.ps1 `
    -WallDirectory release\Wall-v1.0.0-windows-x64-portable `
    -VideoPath C:\path\to\test.mp4
```

复现“恢复上次壁纸”的启动路径时，保留当前应用数据，并让 `VideoPath` 指向会话中正在恢复的媒体：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\verify-portable.ps1 `
    -WallDirectory release\Wall-v1.0.0-windows-x64-portable `
    -VideoPath C:\path\to\active-wallpaper.mp4 `
    -UseExistingData
```

如果验证被强制中断，脚本会保留原数据备份并拒绝继续普通测试。关闭所有 Wall 进程后执行：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\verify-portable.ps1 `
    -WallDirectory release\Wall-v1.0.0-windows-x64-portable `
    -RecoverInterruptedRun
```

## 项目结构

| 路径                   | 职责                                             |
| ---------------------- | ------------------------------------------------ |
| `src/`                 | Vue 3 界面、路由、状态与 Tauri 调用              |
| `src-tauri/src/`       | Windows 窗口、WorkerW、mpv、托盘、存储与系统监视 |
| `scripts/`             | mpv 准备、示例媒体生成、便携发布与宿主验证       |
| `docs/design/`         | 产品范围、界面和视觉规范                         |
| `docs/implementation/` | 产品契约、实现边界和验收证据                     |

## 文档与协作

- [设计规范](docs/design/wall-v1-design-spec.md)
- [产品契约与实现报告](docs/implementation/wall-v1-contract-report.md)
- [贡献指南](CONTRIBUTING.md)
- [安全策略](SECURITY.md)
- [版本记录](CHANGELOG.md)

欢迎提交问题和 Pull Request。使用 AI 辅助开发是允许的，但提交者必须审查代码、运行测试并对许可负责。

## 许可与作者

Wall 源码及项目生成的示例媒体采用 MIT License。便携包中的 mpv 使用其自身许可，详见
`THIRD_PARTY_NOTICES.md`。

- 作者：NiceBlueChai
- 联系邮箱：bluechai@qq.com
