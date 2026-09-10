# SoulMusic

SoulMusic 是一款面向 Windows 的轻量桌面音乐播放器。它将 Soulseek 搜索、音源选择、下载和播放流程整合到简洁的播放器界面中，用户无需单独安装或手动启动 slskd、mpv。

> 请仅搜索、下载和播放你有权使用的内容，并遵守所在地法律法规及 Soulseek 网络规则。

## 主要功能

- 自动启动并连接内置 slskd 服务
- 搜索并聚合 Soulseek 网络中的音频结果
- 按音质、速度和队列情况选择音源
- 下载完成后自动播放，支持 FLAC、MP3、AAC、M4A、OGG、OPUS、WAV、APE、WV
- 下载队列、实时进度、速度和完成状态
- 本地音乐目录扫描与播放
- 播放/暂停、进度跳转、音量、上一首和下一首
- 收藏、播放历史和播放队列
- 亮色界面及单文件、无控制台窗口的 Windows EXE

## 技术栈

- 桌面端：Tauri 2、Rust
- 前端：Vue 3、TypeScript、Vite
- 数据：SQLite（rusqlite）
- 网络服务：slskd / Soulseek
- 音频播放：mpv JSON IPC

## 环境要求

- Windows 10 或 Windows 11（64 位）
- Node.js 20 或更高版本
- Rust stable（MSVC 工具链）
- Visual Studio Build Tools，包含“使用 C++ 的桌面开发”组件
- Microsoft Edge WebView2 Runtime

## 安装依赖（国内源）

项目已在 `.cargo/config.toml` 中配置 `rsproxy.cn`。npm 可临时指定淘宝镜像：

```powershell
npm install --registry=https://registry.npmmirror.com
```

## 本地开发

```powershell
npm run tauri -- dev
```

应用首次启动会将内置 slskd 和 mpv 解压到 `%LOCALAPPDATA%\com.soulmusic.desktop\runtime`，运行数据、缓存和 SQLite 数据库也保存在该应用数据目录中。

## 构建单文件 EXE

无需生成安装包，执行：

```powershell
npm run tauri -- build --no-bundle
```

输出文件：

```text
src-tauri\target\release\SoulMusic.exe
```

Release 模式已使用 Windows GUI 子系统，启动时不会显示控制台黑框。`src-tauri/runtime-assets/` 下的 `slskd.zip` 与 `mpv.7z` 是构建必需资源，请勿删除。

## 项目结构

```text
src/                         Vue 前端界面与 Tauri 调用
src-tauri/src/               Rust 后端、数据库、搜索、下载与播放逻辑
src-tauri/runtime-assets/    构建时嵌入的 slskd、mpv 压缩资源
src-tauri/capabilities/      Tauri 权限配置
.cargo/config.toml           Cargo 国内镜像配置
prd.md                       产品需求文档
```

## 使用说明

1. 启动 SoulMusic，等待左下角显示“Soulseek 已连接”。
2. 输入歌手、歌曲或专辑名称进行搜索。
3. 点击歌曲封面播放，或使用行尾按钮下载、收藏、加入播放队列。
4. 在“本地音乐”中选择文件夹，扫描后可直接播放本地音频。
5. 在“设置”中调整账号、音质偏好、缓存和下载目录。

## 第三方组件

本项目运行时包含 slskd 和 mpv。它们分别遵循各自项目的许可证，本仓库不改变其版权归属。发布或再分发前，请自行核对并满足相应许可证要求。
