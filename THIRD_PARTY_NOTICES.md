<!-- Wall 便携发行包使用的第三方组件和获取源码说明。 -->
# 第三方组件声明

Wall 自身采用 MIT License。便携包另外携带独立进程 `mpv.exe` 及其运行文件，它们不属于 Wall 的 MIT 授权范围。

## mpv Windows build

- 构建：`mpv-x86_64-20260610-git-304426c`
- 发布者：shinchiro/mpv-winbuild-cmake（mpv 官方安装页列出的 Windows 构建）
- 二进制来源：https://github.com/shinchiro/mpv-winbuild-cmake/releases/tag/20260610
- 构建脚本源码：https://github.com/shinchiro/mpv-winbuild-cmake/tree/20260610
- mpv 源码：https://github.com/mpv-player/mpv/tree/304426c
- mpv 许可说明：https://github.com/mpv-player/mpv/blob/master/Copyright

该构建还包含 FFmpeg、libplacebo 等组件。各组件的版本、构建选项和源码地址以对应的
`mpv-winbuild-cmake` 构建脚本为准。重新分发便携包时必须同时保留本文件。

