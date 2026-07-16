<!-- Wall 的开发协作、质量门槛和提交约定。 -->

# 贡献指南

感谢你愿意改进 Wall。v1 的首要约束是 Windows 10/11 x64、本地媒体和完全离线；新增在线服务、账号、
遥测或隐式网络请求前必须先形成明确的产品决策。

## 开发环境

- Node.js 22 与 npm。
- Rust stable。
- Microsoft C++ Build Tools“使用 C++ 的桌面开发”。
- Microsoft Edge WebView2 Runtime。
- 7-Zip；生成正式便携包时还需要 FFmpeg。

```powershell
npm install
powershell -ExecutionPolicy Bypass -File scripts\prepare-mpv.ps1
npm run tauri dev
```

## 修改原则

- 优先复用现有 Vue 组件、CSS 变量、Rust 状态机和命令接口。
- 不引入 Pinia、Tailwind 或 UI 组件库来解决现有原生代码可以完成的问题。
- 不修改用户原媒体；导入操作只记录路径和媒体状态。
- 托盘和前端必须调用同一个 Rust 服务层，不能维护两套播放状态。
- 自动暂停原因使用集合语义，自动条件消失不得覆盖用户的手动暂停。
- 任何新增网络行为必须在 UI、实现、测试和文档中明确说明。

## 编码规范

- UTF-8 无 BOM、CRLF、4 空格缩进，源码单行不超过 120 字符。
- `.editorconfig`、`.gitattributes` 和 `.prettierrc.json` 共同固定基础格式，请不要绕过。
- 所有文件添加文件注释；公共 API 使用 Doxygen 风格注释。
- TypeScript 保持严格类型，Rust 错误转换为统一的可恢复错误结构。
- 提交信息使用中文 Conventional Commit，例如 `fix(player): 修复桌面窗口恢复顺序`。

## 验证

提交前至少运行：

```powershell
npm run test
npm run build
cargo test --manifest-path src-tauri\Cargo.toml
cargo fmt --manifest-path src-tauri\Cargo.toml --all -- --check
```

涉及桌面嵌入、托盘、开机启动或休眠检测时，还应在 Windows 10/11 实机验证。涉及发布包时运行：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\package-portable.ps1
```

## 媒体与第三方许可

- 不提交来源不明、只允许个人使用或禁止再分发的图片、视频、字体和二进制文件。
- 第三方资源必须记录来源、版本、许可和必要的署名。
- 便携包中的三个视频由 `scripts/create-sample-wallpapers.ps1` 生成，不包含第三方画面。
- 更新 mpv 时同步修改准备脚本中的版本、SHA-256 和 `THIRD_PARTY_NOTICES.md`。

## AI 辅助贡献

可以使用 Codex、Claude 等工具，但 AI 不是质量或许可责任的替代者：

- 提交者必须理解并审查最终差异。
- Pull Request 应说明 AI 辅助范围和人工验证内容。
- 不伪造 `Co-authored-by` 身份；只有官方集成提供的有效身份才应写入提交元数据。
- 不向第三方模型提交密钥、私人日志或没有授权的媒体文件。

## Pull Request 清单

- [ ] 变更范围清晰，没有混入无关重构。
- [ ] 新行为有测试，现有测试和构建通过。
- [ ] 用户可见行为、配置或边界已同步更新文档。
- [ ] 新资源具有可验证的再分发许可。
- [ ] 没有引入遥测、更新检查或未说明的网络请求。
