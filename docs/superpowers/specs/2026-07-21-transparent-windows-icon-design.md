<!-- 说明 Wall Windows 任务栏图标黑边修复的素材来源、边界与验证方式。 -->

# 透明 Windows 应用图标

## 目标

保留现有紫色圆角 Wall 标志，移除任务栏图标四角的深色底，并为 Windows 提供可正确缩放的多尺寸 ICO。

## 实现

以现有 `public/icons/app.svg` 为唯一源图，使用项目已安装的 Tauri 图标生成器在临时目录生成标准图标集。
仅同步透明 `src-tauri/icons/icon.png`、多尺寸 `src-tauri/icons/icon.ico` 和 README/便携包使用的
`public/wall-app-icon.png`。

## 边界

不重新设计标志，不使用 AI 重画，不修改托盘状态图标、窗口布局或 Figma。

## 验证

检查 PNG 四角 Alpha 为零、ICO 包含多个 Windows 常用尺寸，并渲染检查缩小后的轮廓；随后运行前端测试与
Tauri 配置检查。
