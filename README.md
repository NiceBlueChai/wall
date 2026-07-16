<!-- Wall 的开发、构建和离线使用说明。 -->
# Wall

Wall 是面向 Windows 10/11 x64 的免费开源动态壁纸工具。它只读取用户选择的本地视频和图片，
不提供账号、商城、云同步、遥测或自动更新。

## 开发

前置环境：Node.js、Rust stable、Microsoft C++ Build Tools 和 Microsoft Edge WebView2 Runtime。

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
powershell -ExecutionPolicy Bypass -File scripts\package-portable.ps1
```

便携包默认假定 Windows 已安装 WebView2 Runtime。应用与壁纸播放本身不需要联网。

## 许可

Wall 源码采用 MIT License。便携包中的 mpv 使用其自身许可，详见 `THIRD_PARTY_NOTICES.md`。

