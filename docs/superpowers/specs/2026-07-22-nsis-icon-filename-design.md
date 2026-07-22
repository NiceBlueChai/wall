<!-- 说明 Wall 安装包图标与公开文件名的发布修复设计。 -->

# NSIS 安装包图标与文件名设计

## 问题

当前 `Wall.exe` 已使用正确的紫色 Wall 图标，但 NSIS 安装包没有显式配置安装器图标，因此回退到 NSIS 默认圆盘
图标。Tauri 内部产物名 `Wall_1.0.0_x64-setup.exe` 还使用下划线，不符合公开发布文件名要求。

## 方案

在 `src-tauri/tauri.conf.json` 中显式声明现有 `icons/icon.ico` 和 `icons/icon.png` 为应用打包图标，并在
`bundle.windows.nsis` 中把 `icons/icon.ico` 同时指定为安装器和卸载器图标。继续复用现有图标资产，不生成新图标。

新增 `scripts/package-installer.ps1` 作为安装版发布入口。脚本读取 `tauri.conf.json` 中的版本号，调用仓库本地 Tauri
CLI 生成 NSIS 包，再把内部产物复制到：

`release/Wall-1.0.0-x64-setup.exe`

Tauri 的内部产物名保持不变；只有公开发布文件使用连字符。便携包命名与流程不改。

## 错误处理

脚本在构建前检查本地 Tauri CLI、ICO 文件、`mpv.exe` 和 `d3dcompiler_43.dll`。任何依赖缺失、构建失败或内部
安装包缺失都会立即终止，不输出旧的发布文件。复制成功后才报告公开安装包路径。

## 验收

1. 配置文件可以被 JSON 解析，PowerShell 脚本可以被解析。
2. 前端测试与构建通过，Rust 格式检查与测试通过。
3. 安装版脚本成功生成 `release/Wall-1.0.0-x64-setup.exe`，文件名不含下划线。
4. 从公开安装包提取的关联图标与 `src-tauri/icons/icon.ico` 都是 Wall 图标，不再是 NSIS 默认图标。
5. 安装包继续包含 `wall.exe`、`mpv.exe` 和 `d3dcompiler_43.dll`，并通过 7-Zip 完整性检查。

安装包代码签名仍不属于本次修复范围。
