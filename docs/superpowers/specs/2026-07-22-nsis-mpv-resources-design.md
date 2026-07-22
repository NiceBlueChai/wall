<!-- 说明 Wall NSIS 安装包携带视频播放依赖的最小修复设计与验收边界。 -->

# NSIS 视频播放依赖修复设计

## 问题

当前 NSIS 安装包只包含 `wall.exe`，没有包含视频播放所需的 `mpv.exe` 和 `d3dcompiler_43.dll`。
便携版脚本会显式复制这两个文件，因此便携版不受影响。

## 方案

在 `src-tauri/tauri.conf.json` 的 `bundle.resources` 中声明两个现有二进制资源，并把它们映射到安装目录根部：

- `binaries/mpv.exe` → `mpv.exe`
- `binaries/d3dcompiler_43.dll` → `d3dcompiler_43.dll`

现有运行时会优先从 `Wall.exe` 所在目录查找 `mpv.exe`，DLL 与 mpv 放在同一目录即可被加载，因此不修改 Rust
播放器逻辑，也不增加依赖或新的打包脚本。

## 错误处理

构建前继续依赖现有 `scripts/prepare-mpv.ps1` 准备二进制文件。缺少资源时让 Tauri 构建直接失败，避免生成不完整的
安装包；运行时保留现有 `mpv_missing` 可恢复错误。

## 验收

1. 前端测试与构建通过，Rust 格式检查与测试通过。
2. Tauri 成功生成 NSIS 安装包。
3. 使用 7-Zip 检查安装包，确认包含 `wall.exe`、`mpv.exe` 和 `d3dcompiler_43.dll`。
4. 在 Windows 实机安装后验证图片壁纸和视频壁纸均可运行。
5. 重新生成便携 ZIP，并确认其完整性和三个关键运行文件仍然齐全。

安装包代码签名不属于本次修复范围；未签名包发布时 Windows 可能显示信誉提示。
