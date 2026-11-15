# Looma Personal Media Tracking Extension v0.1

> **性质：小型功能补丁 / Domain Extension**
>
> 本补丁建立在当前已经完成的：
>
> - Looma v0.2 Architecture Hardening
> - Personal Records / Entertainment / Life / Projects Domain Expansion
>
> 之上。
>
> **不要重新设计 Core、MCP、Audit、Manifest、Entity、Asset、Memory、Collection、Timeline 等基础架构。**
>
> 本补丁只解决一个明确问题：
>
> > Looma 已经能够记录“作品”，但还缺少“我如何消费/管理这个作品”的结构化能力。
>
> 目标是让 Looma 能真正管理：
>
> ```text
> 番剧
> 漫画
> 游戏
> 电影
> TV
> 书籍
> 音乐
> 小说
> ```
>
> 同时允许一个作品同时拥有：
>
> ```text
> 我的观看/阅读/游玩进度
> 本地文件
> 外部网页 / URI
> 我的记忆 / 手记
> 收藏
> 时间线
> ```

---

# 1. 核心目标

当前 Looma 已经可以：

```text
Entity
Asset
Memory
Collection
Timeline
ExternalReference
```

但存在一个实际使用缺口：

```text
BLEACH
```

虽然可以作为 Entity / Work 创建，但是无法自然表达：

```text
我看到第几集？
我看到第几话？
我有没有本地资源？
本地资源在哪里？
我平时从哪个网页看？
这个作品有哪些外部链接？
我什么时候开始看的？
我对它有什么心得？
```

本补丁就是解决这一层。

---

# 2. 最终目标模型

不要创建一个新的 Media 系统。

继续复用：

```text
Work
  │
  ├── Progress
  │
  ├── Assets
  │
  ├── External References
  │
  ├── Memories
  │
  ├── Collections
  │
  └── Timeline
```

目标：

```text
                         Work
                          │
          ┌───────────────┼────────────────┐
          ↓               ↓                ↓
       Progress         Assets          References
          │               │                │
          ↓               ↓                ↓
      我的进度        本地数字资源       外部网页 / URI
          │
          └───────────────┬────────────────┘
                          ↓
                  Personal Records
                  ├── Memory
                  ├── Timeline
                  └── Collection
```

---

# 3. 不要创建独立 Media Database

禁止为了这个功能建立：

```text
AnimeTable
MangaTable
GameTable
MovieTable
AnimeProgressTable
MangaProgressTable
GameProgressTable
```

也不要建立：

```text
MediaManager
AnimeManager
MangaManager
```

作为独立 Core。

继续使用：

```text
Work
Entity
Relation
Asset
Memory
Collection
Timeline
ExternalReference
```

---

# 4. 新增：Progress

这是本补丁的核心。

增加一个通用的：

```text
Progress
```

概念。

它表示：

> **用户当前在一个 Work 上消费到什么位置。**

---

# 5. Progress 最小字段

根据当前项目已有数据模型选择最自然的实现方式。

至少需要能够表达：

```text
work_id
status
position
position_type
updated_at
```

例如：

```text
position_type = episode
position = 24
```

表示：

```text
看到第 24 集
```

或者：

```text
position_type = chapter
position = 520
```

表示：

```text
看到第 520 话
```

---

# 6. Progress Status

至少支持：

```text
planned
in_progress
completed
paused
dropped
revisit
```

例如：

```text
BLEACH
status = in_progress
position_type = episode
position = 3
```

---

# 7. Anime

Anime 使用：

```text
Work
type = anime
```

Progress：

```text
position_type = episode
position = N
```

例如：

```text
BLEACH 千年血战篇

status = in_progress
position_type = episode
position = 12
```

不要创建：

```text
AnimeProgress
```

---

# 8. Manga

Manga：

```text
Work
type = manga
```

Progress：

```text
position_type = chapter
position = N
```

例如：

```text
BLEACH

status = in_progress
position_type = chapter
position = 520
```

---

# 9. Book / Novel

可以支持：

```text
position_type = page
```

或者：

```text
position_type = chapter
```

例如：

```text
Novel
status = in_progress
position_type = chapter
position = 31
```

---

# 10. Game

游戏不强制要求统一的数字进度。

允许：

```text
position_type = custom
position = ...
```

或者使用当前 Entity metadata。

例如：

```text
鸣潮

status = in_progress

metadata:
    version = "..."
    progress = "主线..."
```

不要为了游戏创建：

```text
GameProgress
QuestProgress
AchievementProgress
```

这些暂时不属于本补丁。

---

# 11. Movie

电影通常不需要 Progress。

只需要：

```text
status
Timeline
Memory
```

如果未来需要记录播放时间，再扩展：

```text
position_type = minute
```

本版本不强制。

---

# 12. Progress 更新

必须支持更新：

```text
current position
status
updated_at
```

例如：

```text
BLEACH
episode 12
    ↓
episode 13
```

不要求当前版本建立完整观看历史。

也就是说：

```text
current_position = 13
```

即可。

---

# 13. 是否需要 Progress History

**当前不要求。**

不要为了：

```text
episode 1
episode 2
episode 3
...
```

建立完整历史记录系统。

如果已有 Timeline 能够记录重要节点，可以复用 Timeline。

未来如果确实需要：

```text
“我什么时候看的第 13 集？”
```

再增加 History。

---

# 14. Local Assets

作品必须能够关联现有 Asset。

例如：

```text
BLEACH
│
├── Asset
│   ├── episode01.mp4
│   ├── episode02.mp4
│   ├── screenshot01.png
│   └── poster.jpg
```

不要复制文件。

不要把文件移动进 Looma。

继续使用当前：

```text
Asset
```

以及已有：

```text
path
URI
hash
metadata
```

机制。

---

# 15. Local Asset 的意义

一个 Work 可以：

```text
有本地资源
```

也可以：

```text
完全没有本地资源
```

例如：

```text
BLEACH
├── Local Assets
│   └── 20 files
```

另一个：

```text
某部番
├── Local Assets
│   └── none
```

仍然是合法 Work。

---

# 16. Local Resource Location

如果 Asset 本身已经有：

```text
local_path
uri
```

则直接复用。

不要在 Work 上重复保存：

```text
local_folder
local_path
file_path
```

避免：

```text
Work
 ├── local_path
 └── Asset
      └── local_path
```

造成重复。

---

# 17. Folder-level Association

如果当前 Asset 系统支持目录 / 文件夹资产，则允许：

```text
BLEACH
└── Asset Folder
    └── D:\Anime\BLEACH\
```

如果当前不支持，则不要求本版本实现 Folder Asset。

不要为了这个补丁重新设计 Scanner。

---

# 18. External References

继续使用当前：

```text
ExternalReference
```

不要重复创建第二套 URI 系统。

一个 Work 可以拥有多个 External Reference：

```text
BLEACH

External References
├── Official Website
├── Bilibili
├── AniList
├── Bangumi
└── Other
```

---

# 19. Reference Type

如果当前 ExternalReference 已经存在 type/category，则优先复用。

建议至少能够区分：

```text
official
identity
streaming
reading
wiki
community
source
other
```

例如：

```text
Bilibili
type = streaming
```

```text
Bangumi
type = identity
```

```text
Official Website
type = official
```

---

# 20. 不要把 URI 当作 Asset

重要：

```text
本地文件
    ↓
Asset
```

而：

```text
网页
    ↓
ExternalReference
```

不要把：

```text
https://...
```

伪装成：

```text
Asset
```

除非未来真的实现 Web Resource / Remote Asset。

当前不需要。

---

# 21. External Reference 示例

```text
BLEACH

External References:

1.
title = Bilibili
uri = https://...
type = streaming

2.
title = Official Website
uri = https://...
type = official

3.
title = Bangumi
uri = https://...
type = identity
```

---

# 22. Work Detail Page

这是 UI 最重要的变化。

当前：

```text
Entity Detail
```

应该能够进一步展示 Work 信息。

建议：

```text
┌──────────────────────────────────────┐
│ BLEACH                               │
│ Anime                                │
│                                      │
│ Status: 正在观看                     │
│ Progress: Episode 24                 │
│                                      │
│ [继续观看] [更新进度]                │
├──────────────────────────────────────┤
│ 本地资源                             │
│ 12 个文件                            │
│                                      │
│ [查看资产]                           │
├──────────────────────────────────────┤
│ 外部资源                             │
│ Bilibili                             │
│ Official Website                     │
│ Bangumi                              │
├──────────────────────────────────────┤
│ 我的记录                             │
│ 3 条 Memory                          │
├──────────────────────────────────────┤
│ 所属收藏                             │
│ 我喜欢的番                           │
└──────────────────────────────────────┘
```

具体 UI 风格以当前项目为准。

不要强制照搬这个布局。

---

# 23. Media View

如果当前左侧导航允许增加 View，建议增加：

```text
个人内容
```

而不是：

```text
番剧
漫画
游戏
电影
书籍
音乐
```

五六个独立系统。

推荐：

```text
个人内容
├── 全部作品
├── 番剧
├── 漫画
├── 游戏
├── 电影 / TV
├── 书籍
└── 音乐
```

这些应该是：

> Work Type Filter / View

而不是独立数据库。

---

# 24. Media View

列表至少显示：

```text
Title
Type
Status
Progress
Local Asset Count
External Reference Count
Updated At
```

例如：

```text
BLEACH
Anime
In Progress
Episode 24
12 Assets
3 Links
2026-10-03
```

---

# 25. 快速更新进度

在 Work 卡片上可以提供：

```text
+
-
```

或者：

```text
更新进度
```

例如：

```text
Episode 24
        ↓
Episode 25
```

但不要强制设计复杂播放器。

---

# 26. External URI 快速打开

Work Detail 页面应该允许：

```text
打开 Bilibili
打开官方网站
打开漫画网站
```

使用系统默认浏览器打开 URI。

不要在 Looma 内部嵌入网页。

---

# 27. Asset 与 Work 的双向关系

必须确保：

```text
Work
 ↓
Assets
```

能够查询。

同时：

```text
Asset
 ↓
Related Works
```

也应该可以查询。

例如：

```text
2.png
```

可以显示：

```text
Related:
BLEACH
```

这样继续保持 Looma 的图谱性质。

---

# 28. Memory 与 Work

继续复用现有 Memory。

例如：

```text
BLEACH

Memories:
├── 第一季观后感
├── TYBW 感想
└── 重新观看记录
```

不要创建：

```text
AnimeReview
GamingNote
MangaReview
```

这些都属于 Memory。

---

# 29. Timeline 与 Work

继续复用 Timeline。

例如：

```text
2026-09-01
开始观看 BLEACH

2026-09-25
看完某个篇章

2026-10-03
更新到 Episode 24
```

Progress 表示：

> 当前状态。

Timeline 表示：

> 发生过什么。

两者不要混为一谈。

---

# 30. Collection 与 Work

例如：

```text
Collection
└── 正在看的番
      ├── BLEACH
      ├── ...
```

或者：

```text
Collection
└── 想看的漫画
```

Collection 继续承担：

> 我的分类 / 收藏 / 清单。

不要增加：

```text
AnimeWatchlist
MangaList
GameLibrary
```

---

# 31. MCP

继续使用通用 MCP。

不要增加：

```text
create_anime
create_manga
create_game
update_anime_progress
```

优先复用：

```text
create_entity
update_entity
create_relation
create_memory
create_collection
search
```

Progress 如果当前 Core 暴露为独立 Service，则 MCP 增加：

```text
get_progress
update_progress
```

即可。

---

# 32. MCP Resource

Work Resource 应该能够看到：

```text
looma://entity/<id>
```

中的：

```text
Work
Status
Progress
Assets
ExternalReferences
Memories
Collections
Timeline
```

这样 AI 问：

> “我 BLEACH 看到哪里了？”

可以直接读取 Looma。

---

# 33. AI 场景

至少应该支持：

```text
我 BLEACH 看到第几集了？

我正在看的番有哪些？

我有哪些漫画还没看完？

我有哪些作品有本地资源？

BLEACH 的本地资源在哪里？

BLEACH 有哪些网页入口？

我最近看过什么？

我之前对 BLEACH 写过什么？
```

---

# 34. CLI

继续使用通用命令风格。

如果当前 CLI 已经支持 Entity：

```text
looma entity show BLEACH
```

应该能够看到：

```text
Progress
Assets
References
Memories
Collections
Timeline
```

如果项目 CLI 风格允许，可以增加：

```text
looma progress set <work> <position>
looma progress get <work>
```

但不要强制创建复杂 CLI。

---

# 35. 数据 Migration

必须保持旧数据兼容。

已有：

```text
Entity
Asset
Memory
Collection
Timeline
ExternalReference
```

全部必须继续工作。

如果 Progress 需要新的存储结构：

```text
schema migration
```

必须正常执行。

禁止：

```text
reset database
delete existing records
```

---

# 36. 最小实现范围

### P0 — 必须完成

```text
Progress
├── status
├── position
├── position_type
└── updated_at

Work ↔ Progress

Work ↔ Asset

Work ↔ ExternalReference

Work Detail 展示：
├── Status
├── Progress
├── Local Assets
└── External References
```

---

# 37. P1 — 建议完成

```text
Media View
├── All
├── Anime
├── Manga
├── Game
├── Movie / TV
├── Book
└── Music

Progress 快速更新

External URI 快速打开

Work / Asset 双向查看
```

---

# 38. P2 — 暂不实现

以下内容不要在本补丁中扩张：

```text
Episode Database
Chapter Database
自动追番
自动抓取网页
在线播放器
漫画阅读器
游戏成就系统
Steam API
AniList API
Bangumi API
TMDB API
自动推荐
AI 自动评分
AI 自动生成评论
```

这些属于未来 Provider / Integration / Intelligence。

---

# 39. 一个完整实例

最终应该能够表达：

```text
Work
└── BLEACH 千年血战篇
    type = anime

    Progress
    ├── status = in_progress
    ├── position_type = episode
    ├── position = 24
    └── updated_at = 2026-10-03

    Local Assets
    ├── episode01.mp4
    ├── episode02.mp4
    ├── screenshot01.png
    └── poster.jpg

    External References
    ├── Bilibili
    ├── Official Website
    └── Bangumi

    Memories
    ├── 第一话观后感
    └── TYBW 感想

    Collections
    └── 正在看的番

    Timeline
    ├── 2026-09 开始观看
    └── 2026-10 更新进度
```

---

# 40. 漫画实例

```text
Work
└── BLEACH
    type = manga

    Progress
    ├── status = in_progress
    ├── position_type = chapter
    └── position = 520

    Local Assets
    └── D:\Manga\BLEACH\

    External References
    ├── Reading Website
    └── Bangumi

    Memories
    └── 某章阅读感想

    Collection
    └── 正在看的漫画
```

---

# 41. 游戏实例

```text
Work
└── 鸣潮
    type = game

    Progress
    ├── status = in_progress
    └── position_type = custom

    Local Assets
    ├── screenshot01.png
    └── video01.mp4

    External References
    ├── Official Website
    └── Wiki

    Memories
    └── 游戏体验记录

    Collection
    └── 正在玩的游戏
```

---

# 42. 最终设计原则

本补丁完成后，Looma 应该形成：

```text
                     Work
                      │
          ┌───────────┼───────────┐
          ↓           ↓           ↓
       Progress     Assets    References
          │           │           │
          │           │           │
          └───────────┼───────────┘
                      ↓
               Personal Records
                ├── Memory
                ├── Timeline
                └── Collection
```

也就是说：

> **Work 是“这个东西是什么”。**

> **Progress 是“我到哪里了”。**

> **Asset 是“我本地有什么”。**

> **ExternalReference 是“网上在哪里”。**

> **Memory 是“我怎么想”。**

> **Timeline 是“什么时候发生过”。**

> **Collection 是“我把它归在哪里”。**

这七个概念组合起来，才构成完整的个人媒体记录。

---

# 43. 实现 Agent 最终指令

开始实现前：

```text
1. Inspect current Work / Entity model
2. Inspect current Asset model
3. Inspect current ExternalReference model
4. Inspect current Memory / Collection / Timeline
5. Inspect current GUI navigation and detail pages
6. Reuse existing abstractions wherever possible
```

然后：

```text
P0
→ Progress
→ Work ↔ Progress
→ Work ↔ Asset
→ Work ↔ ExternalReference
→ Work Detail integration
```

再：

```text
P1
→ Media View
→ Quick Progress Update
→ URI Quick Open
→ Bidirectional Asset / Work navigation
```

完成后进行：

```text
Migration Test
Existing Data Test
GUI Test
CLI Test
MCP Test
Backup / Restore Test
```

最终输出：

```text
Implemented
Reused
Changed
Migration
Tests
Remaining Gap
```

---

# 44. 最重要的一句话

**不要把这个补丁实现成“动漫管理器”。**

我们真正要实现的是：

> **Work Tracking Layer**

它应该让 Looma 能够管理：

```text
我正在看什么
我看到哪里
我本地有什么
我从哪里访问
我记录过什么
我什么时候接触过
我把它收藏在哪里
```

而 Anime / Manga / Game / Book / Movie / Music 只是 Work 的不同类型。

这样未来增加任何新的个人数字内容，都不需要重新推翻 Looma。