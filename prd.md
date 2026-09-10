# SoulMusic 桌面音乐播放器 PRD

## 1. 产品概述

### 1.1 产品名称

**SoulMusic**

暂定名称，后续可更换。

### 1.2 产品定位

SoulMusic 是一款基于 Soulseek 网络的桌面音乐播放器。

用户无需理解 Soulseek 的 Peer、队列、远程目录、文件传输等复杂概念，只需要像普通音乐播放器一样：

```text
搜索歌曲
→ 点击播放
→ 自动寻找最佳音乐来源
→ 自动缓冲
→ 开始播放
```

产品在后台完成：

```text
Soulseek 搜索
→ 搜索结果聚合
→ 音频文件识别
→ Peer 质量评分
→ 自动选择最佳来源
→ Soulseek 文件传输
→ Progressive Download
→ 本地缓存
→ mpv 播放
```

产品整体体验应接近：

- Spotify
- Apple Music
- 网易云音乐
- MusicBee

而不是传统 Soulseek 客户端。

---

# 2. 产品目标

## 2.1 核心目标

第一阶段实现最简单、最稳定的音乐播放闭环：

```text
搜索
↓
获得 Soulseek 搜索结果
↓
整理为歌曲
↓
点击歌曲
↓
自动选择最佳 Peer
↓
开始下载
↓
缓存达到播放阈值
↓
自动播放
↓
后台继续下载
```

用户无需：

- 选择 Peer
- 管理传输
- 查看 Soulseek 队列
- 输入远程文件路径
- 手动下载后再打开音乐
- 手动启动 slskd

---

# 3. 产品原则

## 3.1 播放器优先

SoulMusic 首先是一款音乐播放器，其次才是一个 Soulseek 客户端。

所有 UI 和交互围绕：

```text
歌曲
歌手
专辑
播放
收藏
队列
```

设计。

避免让普通用户直接面对：

```text
Peer
Remote Path
Upload Slot
Queue Position
Transfer Token
Soulseek Search ID
```

---

## 3.2 Soulseek 后台化

Soulseek 网络能力作为内部基础设施存在。

用户：

```text
搜索：周杰伦 晴天
```

而不是：

```text
Search Peer Files
```

用户看到：

```text
晴天
周杰伦
叶惠美

FLAC / MP3
12 个可用来源
```

而不是：

```text
username: abc123
remote_path:
D:\Music\Jay Chou\Ye Hui Mei\03.mp3
```

---

## 3.3 自动优于手动

默认：

- 自动寻找音乐
- 自动过滤文件
- 自动选择音质
- 自动选择最快来源
- 自动缓存
- 自动切换备用来源
- 自动预取下一首

高级用户才允许手动选择 Peer。

---

# 4. 目标平台

第一阶段：

```text
Windows 10
Windows 11
```

第二阶段：

```text
macOS
Linux
```

---

# 5. 技术栈

## 5.1 桌面框架

```text
Tauri 2
```

负责：

- 桌面窗口
- 系统托盘
- 文件系统权限
- 启动后台 Sidecar
- Rust 与前端通信
- 应用更新
- 安装包构建

---

## 5.2 前端

```text
Vue 3
TypeScript
Vite
Pinia
Vue Router
```

可选 UI：

```text
Tailwind CSS
shadcn-vue
Radix Vue
```

---

## 5.3 核心后端

```text
Rust
```

负责：

- Soulseek API Adapter
- 搜索结果解析
- 搜索结果聚合
- Source Scoring
- 下载管理
- 播放状态管理
- 缓存管理
- SQLite
- Metadata
- 本地音乐库
- slskd 生命周期
- mpv 生命周期

---

## 5.4 Soulseek 引擎

```text
slskd
```

以 Tauri Sidecar 方式运行。

slskd 不直接暴露给用户。

---

## 5.5 播放器

推荐：

```text
libmpv
```

或者：

```text
mpv.exe Sidecar
```

MVP 优先采用：

```text
mpv IPC
```

后续再切换 libmpv。

---

## 5.6 数据库

```text
SQLite
```

Rust ORM 可采用：

```text
sqlx
```

或：

```text
rusqlite
```

---

# 6. 整体架构

```text
┌────────────────────────────────────────┐
│               SoulMusic                │
│                                        │
│ Vue 3 + TypeScript                     │
│                                        │
│ 首页 / 搜索 / 收藏 / 下载 / 播放队列    │
└──────────────────┬─────────────────────┘
                   │
            Tauri Commands
                   │
┌──────────────────▼─────────────────────┐
│               Rust Core                │
│                                        │
│ SearchService                          │
│ TrackMatcher                           │
│ SourceSelector                         │
│ TransferManager                        │
│ PlaybackService                        │
│ PrefetchManager                        │
│ MetadataService                        │
│ CacheManager                           │
│ LibraryService                         │
│ Database                               │
└──────────────┬───────────────┬─────────┘
               │               │
        HTTP / REST            │ IPC
               │               │
        ┌──────▼──────┐ ┌──────▼─────┐
        │    slskd    │ │     mpv    │
        └──────┬──────┘ └────────────┘
               │
               ▼
       Soulseek Network
               │
         ┌─────┼─────┐
         ▼     ▼     ▼
       Peer A Peer B Peer C
```

---

# 7. 核心模块

系统拆分为：

```text
SoulseekService
SearchService
TrackMatcher
SourceSelector
TransferManager
BufferManager
PlaybackService
QueueManager
PrefetchManager
MetadataService
LibraryService
CacheManager
CoverService
DatabaseService
SettingsService
```

---

# 8. 用户流程

## 8.1 首次启动

启动：

```text
SoulMusic.exe
↓
初始化 AppData
↓
检查 slskd
↓
启动 slskd Sidecar
↓
检查配置
↓
连接 Soulseek
↓
进入首页
```

首次使用进入引导页。

---

# 9. 首次启动引导

页面：

```text
欢迎使用 SoulMusic

连接 Soulseek 网络

用户名
[ SoulMusic_8HD29 ]

密码
[ ************** ]

☑ 自动生成账号

                 开始使用
```

如果没有用户名：

自动生成：

```text
SoulMusic_x8F29K
```

用户以后可修改。

---

# 10. 主界面

整体结构：

```text
┌──────────────────────────────────────────────┐
│ SoulMusic             🔍 搜索歌曲、歌手      │
├──────────┬───────────────────────────────────┤
│          │                                   │
│ 首页     │                                   │
│ 搜索     │            主内容区域             │
│ 收藏     │                                   │
│ 下载     │                                   │
│ 本地音乐 │                                   │
│          │                                   │
├──────────┴───────────────────────────────────┤
│ 封面  晴天 - 周杰伦          ━━━━━━━──────── │
│      ◀     ▶     ▶▶            🔊      ≡    │
└──────────────────────────────────────────────┘
```

---

# 11. 左侧导航

菜单：

```text
首页
搜索
收藏
最近播放
下载
本地音乐
```

底部：

```text
设置
```

---

# 12. 首页

包含：

## 最近播放

显示最近播放过的歌曲。

---

## 最近搜索

例如：

```text
周杰伦
Taylor Swift
Eason Chan
Linkin Park
```

---

## 我的收藏

随机展示：

```text
6～10 首
```

---

## 随机发现

从历史关键词生成 Soulseek 搜索。

例如：

```text
最近经常播放：周杰伦
```

自动搜索：

```text
周杰伦
Jay Chou
周杰伦 live
周杰伦 FLAC
```

构建随机候选池。

---

# 13. 搜索功能

## 13.1 搜索框

用户输入：

```text
周杰伦 晴天
```

点击：

```text
搜索
```

---

# 14. 搜索流程

```text
用户输入关键词
↓
SearchService
↓
slskd Search API
↓
Soulseek Network
↓
多个 Peer 返回
↓
过滤非音频文件
↓
标准化文件名
↓
TrackMatcher 聚合
↓
SourceSelector 计算来源质量
↓
返回前端
```

---

# 15. 搜索过滤

只保留：

```text
.mp3
.flac
.m4a
.aac
.ogg
.opus
.wav
.ape
.wv
```

过滤：

```text
.exe
.zip
.rar
.7z
.jpg
.png
.pdf
.txt
.lrc
.cue
```

其中：

```text
.lrc
.cue
.jpg
```

未来可以作为关联资源处理。

---

# 16. 文件名标准化

例如：

```text
03 - Jay Chou - 晴天 [FLAC].flac
```

转换为：

```text
jay chou 晴天
```

规则：

删除：

```text
01
02
03
[FLAC]
[320K]
CD1
CD2
Disc 1
```

标准化：

```text
全角 → 半角
英文小写
连续空格 → 单空格
特殊字符去除
```

---

# 17. 搜索结果聚合

Soulseek 原始结果：

```text
PeerA:
周杰伦\叶惠美\03 晴天.flac

PeerB:
Jay Chou - Sunny Day.mp3

PeerC:
周杰伦 - 晴天 320K.mp3

PeerD:
03-晴天.flac
```

TrackMatcher 尝试聚合：

```text
晴天
周杰伦
叶惠美
```

来源：

```text
4 Sources
```

---

# 18. 搜索结果 UI

普通模式：

```text
晴天

周杰伦 · 叶惠美

FLAC · 320K MP3
12 个可用来源

                           ▶
```

而不是显示全部 Peer。

---

# 19. 搜索结果排序

综合评分：

```text
TrackScore =
匹配程度 × 0.40
+ 来源数量 × 0.20
+ 最佳 Peer 质量 × 0.20
+ 音质 × 0.10
+ Metadata 完整度 × 0.10
```

优先：

```text
搜索关键词匹配
来源数量多
Peer 在线速度高
音质好
Metadata 完整
```

---

# 20. 来源模型

每个 Track 可以拥有多个 Source。

```typescript
interface TrackSource {
    username: string
    remotePath: string
    filename: string

    size: number

    format: string
    bitrate?: number
    sampleRate?: number
    bitDepth?: number

    uploadSpeed?: number
    queueLength?: number

    score: number
}
```

---

# 21. SourceSelector

给每个来源评分。

基础公式：

```text
SourceScore =
QualityScore × 0.25
+ SpeedScore × 0.30
+ QueueScore × 0.20
+ ReliabilityScore × 0.15
+ CompatibilityScore × 0.10
```

---

# 22. QualityScore

大致优先级：

```text
FLAC
ALAC
APE
WAV
320K MP3
256K AAC
256K MP3
192K MP3
128K MP3
```

但不意味着永远选择 FLAC。

---

# 23. 下载速度评分

例如：

```text
>10 MB/s     100
5～10 MB/s    90
2～5 MB/s     80
1～2 MB/s     70
500K～1MB/s   55
100～500K/s   30
<100KB/s      10
```

---

# 24. QueueScore

```text
Queue 0      100
Queue 1-2     90
Queue 3-5     75
Queue 6-10    50
Queue 11-30   25
Queue >30     5
```

---

# 25. 播放流程

用户点击：

```text
▶ 晴天
```

执行：

```text
检查本地缓存
↓
已缓存？
├─ 是 → 直接播放
└─ 否
    ↓
选择最佳 Source
    ↓
请求 Soulseek 下载
    ↓
检查下载状态
    ↓
文件开始写入
    ↓
监控 Buffer
    ↓
达到启动阈值
    ↓
mpv 打开文件
    ↓
继续下载
```

---

# 26. 播放状态机

```text
IDLE
↓
SEARCHING_SOURCE
↓
SOURCE_SELECTED
↓
CONNECTING
↓
QUEUED
↓
DOWNLOADING
↓
BUFFERING
↓
PLAYING
↓
COMPLETE
```

异常：

```text
PEER_LOST
↓
FIND_ALTERNATIVE
↓
SOURCE_SWITCH
↓
BUFFERING
↓
PLAYING
```

---

# 27. UI 状态

用户看到：

```text
正在寻找音源…
```

之后：

```text
正在连接…
```

如果排队：

```text
当前音源繁忙，正在寻找更快来源…
```

下载：

```text
正在缓冲…
```

播放：

```text
正在播放
```

断线：

```text
正在切换备用音源…
```

避免：

```text
Peer disconnected
Queue position 27
```

直接暴露。

---

# 28. Progressive Download

核心策略：

Soulseek 持续写：

```text
晴天.mp3.part
```

mpv 在文件未下载完成之前开始读取。

状态：

```text
[已下载.................]
[已播放......]
```

只要：

```text
下载进度 > 播放进度 + 安全 Buffer
```

即可继续播放。

---

# 29. BufferManager

动态计算启动阈值。

公式：

```text
EstimatedBytesPerSecond =
Bitrate / 8
```

例如：

```text
320Kbps

320 / 8
≈ 40 KB/s
```

目标缓冲：

```text
15 秒
```

理论：

```text
600KB
```

但实际设置最低阈值。

---

# 30. Buffer 默认设置

建议：

```text
MP3 / AAC / OGG
2 MB

FLAC / ALAC
4 MB

APE
5 MB

WAV
8 MB
```

后续动态调整。

---

# 31. 下载速度安全判断

计算：

```text
DownloadRate
PlaybackRate
```

要求：

```text
DownloadRate >= PlaybackRate × 1.5
```

如果：

```text
DownloadRate < PlaybackRate
```

持续：

```text
5 秒
```

则触发：

```text
寻找备用 Source
```

---

# 32. Peer 切换

如果当前 Peer：

```text
速度下降
断线
排队
传输失败
```

SourceSelector 选择第二名。

状态：

```text
Source A
↓
异常
↓
Source B
↓
重新建立下载
```

---

# 33. 文件一致性

不同来源不能直接拼接。

只有确定：

```text
Size 相同
Hash 相同
```

或者足够可靠确认文件完全相同：

```text
Size
Codec
Bitrate
Duration
Audio Fingerprint
```

才允许续接。

否则：

```text
重新建立缓存文件
```

例如：

```text
cache/
track123-sourceA.part
track123-sourceB.part
```

---

# 34. 播放器

功能：

```text
播放
暂停
上一首
下一首
Seek
音量
静音
循环
单曲循环
随机播放
```

---

# 35. PlayerBar

底部固定：

```text
┌────────────────────────────────────────────────────┐
│ [封面] 晴天                                        │
│        周杰伦        ◀  ▶  ▶▶     ━━━━━────  🔊 ≡ │
└────────────────────────────────────────────────────┘
```

---

# 36. Seek 限制

如果文件尚未下载完成：

```text
播放器只能 Seek 到已经下载区域
```

例如：

```text
歌曲：
0 ------------------------- 240s

已下载：
0 ------------ 110s

已播放：
0 ----- 50s
```

用户拖到：

```text
180s
```

则：

```text
提示正在缓冲目标位置
```

MVP 可以禁止 Seek 超出缓存范围。

---

# 37. 播放队列

Queue：

```text
晴天
七里香
夜曲
一路向北
稻香
```

支持：

```text
拖动排序
删除
立即播放
下一首播放
清空
```

---

# 38. PrefetchManager

当前播放：

```text
Track A
```

同时：

```text
预取 Track B
预取 Track C
```

默认：

```text
PrefetchCount = 2
```

---

# 39. Prefetch 状态

```text
PLAYING
Track A

PREFETCH
Track B

WAITING
Track C
```

Track A 剩：

```text
30 秒
```

如果 Track B 还没有满足缓存条件：

提高：

```text
Track B 下载优先级
```

---

# 40. 搜索页

布局：

```text
🔍 周杰伦

歌曲     专辑      文件
──────────────────────────

晴天
周杰伦 · 叶惠美
12 Sources
FLAC / MP3

▶      ♡      ⋯
```

---

# 41. 高级来源页面

点击：

```text
⋯
```

选择：

```text
查看来源
```

页面：

```text
晴天

来源             质量       速度       队列

music123         FLAC       8.2MB/s     0
jayfan           MP3 320K   6.3MB/s     0
lossless         FLAC       900KB/s     8
abc123           MP3 256K   2.1MB/s     1
```

操作：

```text
使用此来源
复制文件名
查看用户共享
```

最后一项属于高级功能，可延后。

---

# 42. 收藏功能

用户点击：

```text
♡
```

保存：

```text
Track Metadata
```

而不是保存当前 Peer。

以后再次播放：

```text
重新搜索最佳来源
```

因此：

```text
收藏的是歌曲
```

而不是：

```text
收藏 Peer 文件
```

---

# 43. 收藏数据

```text
track_id
title
artist
album
duration
cover
created_at
```

---

# 44. 最近播放

记录：

```text
track_id
played_at
source
completed_ratio
```

例如：

```text
晴天
周杰伦
刚刚

夜曲
周杰伦
15 分钟前
```

---

# 45. 下载功能

播放与下载必须分开。

播放：

```text
Soulseek
↓
Cache
↓
播放
↓
自动清理
```

下载：

```text
Soulseek
↓
Downloads
↓
永久保存
```

---

# 46. 下载按钮

搜索结果：

```text
⬇
```

点击：

```text
下载
```

用户可选择：

```text
自动最佳音质
FLAC 优先
320K MP3 优先
选择来源
```

---

# 47. 下载页面

显示：

```text
正在下载
已完成
失败
```

卡片：

```text
晴天
周杰伦

42 MB / 61 MB
8.2 MB/s

██████████████────── 68%

暂停
取消
```

---

# 48. 本地音乐库

扫描：

```text
~/Music/SoulMusic
```

支持：

```text
MP3
FLAC
AAC
M4A
OGG
OPUS
APE
WAV
```

页面：

```text
歌曲
歌手
专辑
文件夹
```

---

# 49. MetadataService

读取：

```text
ID3v2
Vorbis Comments
FLAC Metadata
MP4 Tags
APEv2
```

提取：

```text
title
artist
album
album_artist
track
disc
year
genre
duration
bitrate
sample_rate
bit_depth
```

---

# 50. Metadata 优先级

```text
文件 Tag
↓
文件名解析
↓
MusicBrainz
↓
搜索关键词
```

不要过度依赖文件名。

---

# 51. 封面

优先：

```text
音频内嵌封面
↓
本地 folder.jpg
↓
MusicBrainz
↓
Cover Art Archive
↓
默认封面
```

---

# 52. 默认封面

没有封面时：

```text
彩色渐变背景
+
音乐符号
```

并根据：

```text
title + artist
```

生成固定渐变 Seed。

---

# 53. 缓存目录

Windows：

```text
%LOCALAPPDATA%\SoulMusic\
```

结构：

```text
SoulMusic/
├── cache/
│   ├── audio/
│   ├── covers/
│   ├── metadata/
│   └── search/
│
├── downloads/
│
├── runtime/
│   ├── slskd/
│   └── mpv/
│
├── logs/
│
├── soulmusic.db
└── config.json
```

---

# 54. Audio Cache

文件：

```text
cache/audio/
```

命名：

```text
{track_id}_{source_id}.part
```

完整：

```text
{track_id}.mp3
```

---

# 55. CacheManager

默认：

```text
最大音频缓存：

10 GB
```

用户可设置：

```text
2 GB
5 GB
10 GB
20 GB
50 GB
无限制
```

---

# 56. 缓存清理

算法：

```text
LRU
```

优先删除：

```text
很久没有播放
未收藏
非下载
```

永远不自动删除：

```text
Downloads
```

---

# 57. 搜索缓存

同一个：

```text
周杰伦 晴天
```

一定时间内不重复发起 Soulseek 搜索。

TTL：

```text
30～120 秒
```

同时允许：

```text
刷新搜索
```

---

# 58. slskd 生命周期

Tauri 启动：

```text
检查端口
↓
检查 slskd process
↓
启动 Sidecar
↓
等待 Health Check
↓
连接 API
```

退出：

```text
停止未必要的搜索
↓
保存播放状态
↓
关闭 mpv
↓
通知 slskd shutdown
↓
kill fallback
```

---

# 59. slskd 配置

由应用自动生成。

例如：

```text
runtime/slskd/slskd.yml
```

包含：

```text
Soulseek Username
Soulseek Password
Listen Port
Downloads Directory
Incomplete Directory
API Key
Localhost Binding
```

API 仅：

```text
127.0.0.1
```

监听。

---

# 60. slskd 安全

禁止默认：

```text
0.0.0.0
```

建议：

```text
localhost only
```

API Key：

```text
首次启动随机生成
```

例如：

```text
64 byte random token
```

---

# 61. Rust Adapter

不要让前端直接依赖 slskd。

定义：

```rust
trait SoulseekProvider {
    async fn search(
        &self,
        query: &str
    ) -> Result<SearchSession>;

    async fn results(
        &self,
        search_id: &str
    ) -> Result<Vec<SearchResult>>;

    async fn download(
        &self,
        source: &TrackSource
    ) -> Result<TransferId>;

    async fn transfer(
        &self,
        id: &TransferId
    ) -> Result<TransferStatus>;
}
```

目前实现：

```text
SlskdProvider
```

以后可以实现：

```text
SoulseekNetProvider
NativeSoulseekProvider
```

---

# 62. Tauri Commands

前端只调用：

```text
search_tracks
play_track
pause
resume
seek
next
previous
add_to_queue
remove_from_queue
favorite_track
download_track
cancel_download
get_downloads
get_library
get_settings
update_settings
```

---

# 63. Search API

例如：

```typescript
invoke("search_tracks", {
    query: "周杰伦 晴天"
})
```

返回：

```json
{
  "searchId": "abc",
  "tracks": [
    {
      "id": "track_123",
      "title": "晴天",
      "artist": "周杰伦",
      "album": "叶惠美",
      "formats": ["FLAC", "MP3"],
      "sourceCount": 12
    }
  ]
}
```

---

# 64. 播放 API

```typescript
invoke("play_track", {
    trackId: "track_123"
})
```

返回：

```json
{
  "status": "buffering"
}
```

---

# 65. Events

Rust → Vue：

```text
player://state
player://position
player://buffer
player://track
transfer://progress
transfer://speed
search://updated
queue://updated
```

---

# 66. Player Store

Pinia：

```typescript
PlayerStore {
    currentTrack

    state
    duration
    position
    volume

    bufferedUntil

    queue
}
```

---

# 67. Search Store

```typescript
SearchStore {
    query
    loading

    tracks

    activeSearchId
}
```

---

# 68. 数据库设计

## tracks

```sql
CREATE TABLE tracks (
    id TEXT PRIMARY KEY,

    title TEXT,
    artist TEXT,
    album TEXT,

    duration INTEGER,

    cover_path TEXT,

    created_at INTEGER,
    updated_at INTEGER
);
```

---

# 69. sources

```sql
CREATE TABLE sources (
    id TEXT PRIMARY KEY,

    track_id TEXT,

    username TEXT,
    remote_path TEXT,
    filename TEXT,

    size INTEGER,

    format TEXT,
    bitrate INTEGER,
    sample_rate INTEGER,
    bit_depth INTEGER,

    upload_speed INTEGER,
    queue_length INTEGER,

    score REAL,

    last_seen INTEGER
);
```

---

# 70. favorites

```sql
CREATE TABLE favorites (
    track_id TEXT PRIMARY KEY,
    created_at INTEGER
);
```

---

# 71. play_history

```sql
CREATE TABLE play_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    track_id TEXT,

    played_at INTEGER,

    duration_played INTEGER,
    completion_ratio REAL
);
```

---

# 72. queue

```sql
CREATE TABLE queue (
    position INTEGER PRIMARY KEY,
    track_id TEXT
);
```

---

# 73. downloads

```sql
CREATE TABLE downloads (
    id TEXT PRIMARY KEY,

    track_id TEXT,
    source_id TEXT,

    status TEXT,

    downloaded_bytes INTEGER,
    total_bytes INTEGER,

    local_path TEXT,

    created_at INTEGER,
    completed_at INTEGER
);
```

---

# 74. search_history

```sql
CREATE TABLE search_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    query TEXT,

    searched_at INTEGER
);
```

---

# 75. settings

```sql
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT
);
```

---

# 76. 推荐项目目录

```text
soulmusic/
│
├── src/
│   ├── pages/
│   │   ├── Home.vue
│   │   ├── Search.vue
│   │   ├── Favorites.vue
│   │   ├── Downloads.vue
│   │   ├── Library.vue
│   │   └── Settings.vue
│   │
│   ├── components/
│   │   ├── SearchBar.vue
│   │   ├── TrackRow.vue
│   │   ├── AlbumCard.vue
│   │   ├── PlayerBar.vue
│   │   ├── QueueDrawer.vue
│   │   ├── SourceDrawer.vue
│   │   └── DownloadRow.vue
│   │
│   ├── stores/
│   │   ├── player.ts
│   │   ├── search.ts
│   │   ├── queue.ts
│   │   ├── library.ts
│   │   └── settings.ts
│   │
│   └── services/
│
├── src-tauri/
│   ├── src/
│   │   ├── commands/
│   │   │
│   │   ├── soulseek/
│   │   │   ├── mod.rs
│   │   │   ├── client.rs
│   │   │   ├── search.rs
│   │   │   ├── transfer.rs
│   │   │   └── model.rs
│   │   │
│   │   ├── playback/
│   │   │   ├── player.rs
│   │   │   ├── queue.rs
│   │   │   └── prefetch.rs
│   │   │
│   │   ├── metadata/
│   │   │
│   │   ├── library/
│   │   │
│   │   ├── cache/
│   │   │
│   │   ├── database/
│   │   │
│   │   └── main.rs
│   │
│   └── binaries/
│       ├── slskd.exe
│       └── mpv.exe
│
└── package.json
```

---

# 77. 设置页面

分组：

## Soulseek

```text
用户名
密码

连接状态

● 已连接
```

操作：

```text
重新连接
```

---

## 音质

```text
自动

☑ 优先无损
☑ 优先 FLAC

最低 MP3 码率：
192K
```

模式：

```text
速度优先
平衡
音质优先
```

---

# 78. Source Selector 模式

## 速度优先

```text
Speed 50%
Queue 25%
Quality 15%
Reliability 10%
```

---

## 平衡

```text
Speed 30%
Quality 25%
Queue 20%
Reliability 15%
Compatibility 10%
```

---

## 音质优先

```text
Quality 50%
Speed 20%
Queue 15%
Reliability 10%
Compatibility 5%
```

---

# 79. 播放设置

```text
预缓冲：

自动
5 秒
10 秒
15 秒
30 秒
```

默认：

```text
自动
```

---

# 80. Prefetch

设置：

```text
预加载下一首

0
1
2
3
```

默认：

```text
2
```

---

# 81. 缓存设置

```text
缓存：

10GB
```

显示：

```text
已使用：

4.2 GB / 10 GB
```

按钮：

```text
清理缓存
```

---

# 82. 下载设置

默认目录：

```text
C:\Users\User\Music\SoulMusic
```

配置：

```text
下载完成后自动整理目录
```

例如：

```text
Artist/
└── Album/
    └── Track.mp3
```

---

# 83. 系统托盘

关闭窗口：

默认：

```text
最小化到托盘
```

托盘菜单：

```text
SoulMusic

晴天 - 周杰伦

播放 / 暂停
上一首
下一首

显示窗口

退出
```

---

# 84. Windows Media Integration

第二阶段接入：

```text
Windows System Media Transport Controls
```

支持：

```text
键盘媒体键
锁屏播放器
Windows 音量面板媒体控制
```

---

# 85. 异常处理

## Soulseek 未连接

显示：

```text
Soulseek 网络暂时不可用

[重新连接]
```

---

# 86. 无搜索结果

```text
没有找到音乐

建议：
• 简化关键词
• 只搜索歌手 + 歌名
• 尝试英文名
```

---

# 87. Peer 无法连接

后台：

```text
切换备用 Source
```

如果全部失败：

```text
当前没有可播放来源

[重新搜索]
```

---

# 88. 下载太慢

如果：

```text
Peer Speed < Playback Speed
```

显示：

```text
当前音源速度较慢

正在寻找更快来源…
```

---

# 89. 播放中断

如果 Buffer：

```text
< 2 秒
```

暂停：

```text
Buffering
```

恢复达到：

```text
5 秒
```

继续播放。

---

# 90. 下载失败

自动：

```text
Retry 1
↓
Retry 2
↓
Alternative Source
```

最多：

```text
3 Sources
```

---

# 91. 日志

保存：

```text
logs/app.log
logs/slskd.log
logs/player.log
```

正常情况下 UI 不显示。

设置：

```text
导出诊断日志
```

方便 Debug。

---

# 92. 隐私与安全

SoulMusic 不应该默认把：

```text
API
slskd Web UI
```

暴露到局域网。

仅：

```text
127.0.0.1
```

---

# 93. 密码存储

Soulseek 密码不要明文保存到：

```text
config.json
```

Windows 使用：

```text
Windows Credential Manager
```

或者：

```text
Tauri Stronghold
```

---

# 94. 文件安全

Soulseek 搜索结果仅允许打开白名单媒体格式。

禁止：

```text
.exe
.bat
.cmd
.ps1
.msi
.js
.vbs
.scr
```

通过播放器执行。

---

# 95. 版权提示

首次启动：

```text
SoulMusic 是 Soulseek 网络客户端。

Soulseek 中的资源由网络用户提供。

请仅访问、播放或下载你拥有合法访问权的内容，并遵守所在地区法律法规。
```

用户确认：

```text
我知道了
```

---

# 96. Soulseek Radio

第二阶段功能。

用户搜索：

```text
周杰伦
```

点击：

```text
随机播放
```

系统建立：

```text
Candidate Pool
```

例如：

```text
晴天
夜曲
七里香
搁浅
稻香
一路向北
```

---

# 97. Radio 算法

```text
Search
↓
Collect
↓
TrackMatcher
↓
Remove Duplicates
↓
Remove Recently Played
↓
Shuffle
↓
SourceSelector
↓
Prefetch
```

候选池保持：

```text
20～50 首
```

低于：

```text
10
```

自动继续搜索。

---

# 98. 智能搜索扩展

例如：

```text
周杰伦
```

生成：

```text
周杰伦
Jay Chou
周杰伦 FLAC
Jay Chou FLAC
周杰伦 live
```

但 MVP 不做 LLM。

完全使用：

```text
规则
Metadata
历史结果
```

完成。

---

# 99. 下载去重

计算：

```text
Size
Duration
Codec
Bitrate
Audio Fingerprint
```

检测重复文件。

未来可加入：

```text
Chromaprint
AcoustID
```

---

# 100. Track ID

建议生成：

```text
hash(
normalize(title)
+
normalize(artist)
+
round(duration)
)
```

比如：

```text
track_8c7adf...
```

未知 Metadata 时：

暂时：

```text
hash(filename + size)
```

下载后再合并 Track。

---

# 101. 搜索匹配算法

例如：

```text
Query:

周杰伦 晴天
```

候选：

```text
周杰伦 - 晴天.flac
```

计算：

```text
Token Match
Fuzzy Match
Filename Similarity
Artist Match
Title Match
```

最终：

```text
MatchScore 0～100
```

低于：

```text
50
```

默认隐藏。

---

# 102. Performance

目标：

应用启动：

```text
< 3 秒
```

slskd 后台可在：

```text
3～10 秒
```

内完成网络连接。

UI 不阻塞。

---

# 103. 搜索响应

目标：

```text
0～1 秒：

显示正在搜索
```

随着 Peer 返回：

实时追加：

```text
3 Results
↓
20 Results
↓
86 Results
```

而不是等待整个 Soulseek 搜索完成。

---

# 104. 虚拟列表

搜索可能产生：

```text
数百
数千
```

结果。

前端必须采用：

```text
Virtual List
```

避免大量 DOM。

---

# 105. 并发搜索限制

建议：

```text
Concurrent Search <= 3
```

用户再次搜索：

旧 Search 可以：

```text
取消 UI 订阅
```

后台等待自然结束或主动取消。

---

# 106. Transfer Scheduler

优先级：

```text
P0 当前播放
P1 下一首预取
P2 再下一首预取
P3 用户主动下载
P4 Cache
```

当前播放永远优先。

---

# 107. 带宽控制

设置：

```text
下载速度限制
```

默认：

```text
无限制
```

用户可选择：

```text
1 MB/s
5 MB/s
10 MB/s
20 MB/s
无限制
```

---

# 108. 播放结束处理

如果歌曲来自 Cache：

```text
更新 play_history
↓
增加 Cache LastAccess
↓
播放下一首
```

如果下载仍未完成：

可以继续：

```text
完成下载
```

或：

```text
取消
```

建议：

只剩：

```text
<20%
```

继续完成。

剩余：

```text
>50%
```

且用户跳过，则取消低优先级传输。

---

# 109. MVP 功能范围

版本：

```text
v0.1
```

必须完成：

```text
Soulseek 登录
slskd Sidecar
搜索
音频文件过滤
搜索结果展示
自动选源
Soulseek 下载
本地 Buffer
mpv 播放
播放 / 暂停
播放进度
下一首
播放队列
缓存
SQLite
设置
```

---

# 110. v0.2

增加：

```text
收藏
最近播放
下载管理
本地音乐库
Metadata
封面
Source 手动选择
```

---

# 111. v0.3

增加：

```text
Prefetch
动态 Buffer
自动切换 Source
Soulseek Radio
系统媒体控制
托盘
```

---

# 112. v1.0

目标：

```text
像普通在线播放器一样使用 Soulseek
```

用户无需感知：

```text
下载
Peer
Queue
Remote Path
```

大多数歌曲达到：

```text
点击后几秒内开始播放
```

---

# 113. MVP 开发顺序

建议严格按照以下顺序：

```text
1. Tauri + Vue 项目

2. 启动 slskd Sidecar

3. Rust 调用 slskd API

4. Soulseek Search

5. 展示原始搜索结果

6. 音频过滤

7. SourceSelector

8. 下载单个文件

9. mpv 播放完整文件

10. Progressive Download

11. BufferManager

12. PlayerBar

13. Queue

14. Prefetch

15. SQLite

16. CacheManager

17. Metadata

18. 封面

19. 收藏 / 历史

20. 安装包
```

不要第一阶段直接做：

```text
推荐算法
歌词
聊天室
好友
Soulseek Rooms
用户浏览
完整共享功能
```

---

# 114. MVP 验收标准

## 搜索

输入：

```text
Artist + Song
```

能够：

```text
从 Soulseek 获得结果
```

并且：

```text
非音乐文件被过滤
```

---

## 播放

点击歌曲：

```text
自动选择 Source
```

无需用户选择 Peer。

缓存满足条件后：

```text
自动开始播放
```

---

## 中断

如果 Peer 掉线：

```text
尝试重新寻找 Source
```

而不是直接 App 崩溃。

---

## Queue

添加：

```text
3 首歌曲
```

能够：

```text
连续播放
```

---

## Cache

播放过的歌曲：

```text
再次播放优先本地缓存
```

---

# 115. 产品最终体验

最终用户操作应该只有：

```text
打开 SoulMusic
        ↓
搜索“周杰伦 晴天”
        ↓
看到“晴天 - 周杰伦”
        ↓
点击 ▶
        ↓
正在寻找最佳音源…
        ↓
正在缓冲…
        ↓
开始播放
```

而底层实际上完成：

```text
Soulseek Search
        ↓
几十个 Peer Results
        ↓
Audio Filter
        ↓
TrackMatcher
        ↓
SourceSelector
        ↓
Peer Connection
        ↓
Soulseek Transfer
        ↓
Progressive Download
        ↓
BufferManager
        ↓
mpv
        ↓
Prefetch
```

这正是 SoulMusic 的核心价值：

> **把复杂的 Soulseek P2P 文件网络，包装成简单、现代、接近流媒体体验的桌面音乐播放器。**

---

# 116. 最终技术架构

```text
                       SoulMusic
                           │
                    Vue 3 + Tauri
                           │
                      Rust Core
                           │
         ┌─────────────────┼────────────────┐
         │                 │                │
         ▼                 ▼                ▼
   SearchService     PlaybackService     SQLite
         │                 │
         ▼                 ▼
   TrackMatcher            mpv
         │
         ▼
   SourceSelector
         │
         ▼
       slskd
         │
         ▼
  Soulseek Network
         │
 ┌───────┼────────┐
 ▼       ▼        ▼
Peer A  Peer B   Peer C
         │
         ▼
  Progressive Download
         │
         ▼
      Cache
         │
         ▼
      Player
```

## 技术栈最终确定

```text
Desktop:
Tauri 2

Frontend:
Vue 3
TypeScript
Vite
Pinia

Backend:
Rust

Soulseek:
slskd

Audio:
mpv / libmpv

Database:
SQLite

Metadata:
ID3 / FLAC / Vorbis / MP4 Tags

Cover:
Embedded Cover
+ MusicBrainz
+ Cover Art Archive

Platform:
Windows 10 / 11
```

## 一句话产品定义

**SoulMusic = Soulseek 网络搜索 + 自动最佳音源选择 + Progressive Download + 本地缓存 + mpv 播放器。**