# Looma Domain Expansion Pack

## Personal Records / Entertainment / Life / Projects

> **性质：扩展包**
>
> 本文不是新的 Architecture Hardening 主任务。
>
> Looma v0.2 Architecture Hardening 已经完成后，本扩展包用于在现有架构之上增加“个人记录领域”的表达能力。
>
> 核心目标：
>
> **让 Looma 真正能够长期记录一个人的数字生活，而不仅仅是管理文件和通用 Entity。**

---

# 1. 重要约束

本扩展包必须建立在当前已经完成的 Looma v0.2 架构之上。

不要重新设计：

* LoomaCore
* MCP Permission
* MCP Resources
* Audit
* Manifest
* CloudStorage
* Domain Event
* Repository
* Backup

这些能力已经属于基础设施。

本任务只负责：

> **Domain Expansion**

---

# 2. 不要把 Looma 做成垂直 App

尤其不要把 Looma 变成：

```text
AniList Clone
MyAnimeList Clone
Steam Library Clone
Goodreads Clone
Notion Clone
Letterboxd Clone
```

我们真正需要的是：

```text
Personal Digital OS
        │
        ├── Entertainment
        ├── Projects
        ├── Life
        ├── Knowledge
        └── Digital Assets
```

因此：

> Anime / Game / Movie / Book 等都应该是 Looma Personal Record Domain 的具体类型，而不是独立系统。

---

# 3. 核心 Domain 模型

继续复用现有：

```text
Asset
Entity
Relation
Memory
Collection
Timeline
ExternalReference
```

不重复制造平行模型。

推荐关系：

```text
                         Entity
                            │
             ┌──────────────┼──────────────┐
             ↓              ↓              ↓
            Work          Person         Project
             │
      ┌──────┼──────┬──────┬──────┐
      ↓      ↓      ↓      ↓      ↓
    Anime   Manga   Game   Movie   Book
      │
      ↓
   Memories
      │
      ↓
   Timeline
      │
      ↓
    Assets
      │
      ↓
External References
```

---

# 4. Work

增加一个通用的：

```text
Work
```

概念。

Work 表示一个被用户消费、关注、收藏、记录或研究的作品。

例如：

```text
Anime
Manga
Game
Movie
TV Series
Music
Album
Book
Novel
Documentary
```

---

# 5. Work Type

建议至少支持：

```text
anime
manga
game
movie
tv_series
music
album
book
novel
documentary
other
```

具体实现可以根据当前 Entity 模型决定。

不要为了这些类型建立十套独立数据库表。

---

# 6. Work Metadata

一个 Work 至少能够表达：

```text
id
title
type
original_title
description
status
start_date
end_date
release_year
cover_asset
```

不要求一次把所有字段全部实现。

优先保证：

```text
title
type
status
date
```

能够表达。

---

# 7. Status

不同领域可以共享一套基础状态：

```text
planned
in_progress
completed
paused
dropped
revisit
unknown
```

例如：

```text
BLEACH
status = completed / in_progress

某个游戏
status = in_progress

某本书
status = planned
```

如果某个领域需要更加专业的状态，再通过 metadata 扩展。

不要为每个类型创建完全不同的状态系统。

---

# 8. Personal Consumption Record

这是本扩展包最重要的概念之一。

Looma 不只是记录：

> “我知道这个作品存在。”

还应该能够记录：

> “我什么时候接触过它。”

因此允许 Work 与 Timeline / Memory 建立关系。

例如：

```text
BLEACH
 │
 ├── discovered: 2025
 ├── started: 2026-08
 ├── watched: 2026-09
 └── revisited: 2026-10
```

---

# 9. 不强制建立 WatchHistory 表

不要直接设计成：

```text
watch_history
episode_history
anime_progress
```

这些属于垂直应用设计。

Looma 更推荐：

```text
Work
  ↓
Timeline Event
  ↓
Memory
```

例如：

```text
Timeline
2026-09-25
    ↓
Started BLEACH TYBW

Memory
2026-09-25
    ↓
“这一段剧情让我觉得……”
```

这样未来游戏、电影、书籍都可以复用。

---

# 10. Episode / Chapter / Volume

当前阶段不要强制实现完整：

```text
Episode
Chapter
Volume
Season
```

如果现有 Entity / Relation 已经能够表达：

```text
BLEACH
 └── TYBW
      └── Episode / Season
```

则优先使用 Entity + Relation。

只有实际使用中证明需要结构化进度时，再增加专用模型。

---

# 11. Collection

Collection 在 Personal Records 中非常重要。

例如：

```text
我的番剧
正在看的番
看完的番
想看的番
最喜欢的作品
2026 年看过的作品
游戏收藏
```

这些都不应该写死成数据库类型。

应该使用：

```text
Collection
    ↓
references Entity / Work
```

---

# 12. Example：Anime

例如：

```text
Entity
└── BLEACH
    type = anime

Collection
└── 我喜欢的番

Timeline
├── 2026-08
│   └── 开始重看
└── 2026-09
    └── 看完 TYBW

Memory
└── 2026-09
    └── 对某段剧情的个人感受

Asset
├── screenshot
└── image

ExternalReference
├── Bilibili
└── Website
```

这就是目标使用方式。

---

# 13. Manga

漫画复用 Work：

```text
Entity
└── BLEACH
    type = manga
```

可以通过 Relation：

```text
BLEACH manga
       │
       └── adaptation_of
               ↓
          BLEACH anime
```

不要创建：

```text
anime_bleach
manga_bleach
```

这样的特殊结构。

---

# 14. Game

游戏同样使用：

```text
Work
type = game
```

可以关联：

```text
Game
 ├── Platform
 ├── Developer
 ├── Publisher
 ├── Character
 ├── Project
 ├── Memory
 ├── Screenshot
 └── ExternalReference
```

例如：

```text
Game
└── 鸣潮

Assets
├── Screenshot
└── Video

Memories
└── 游戏体验记录

ExternalReferences
└── Official Website
```

---

# 15. Movie / TV

继续复用：

```text
Work
type = movie
```

或：

```text
Work
type = tv_series
```

例如：

```text
电视剧
 ├── Work
 ├── Timeline
 ├── Memory
 ├── Collection
 └── ExternalReference
```

不需要新的系统。

---

# 16. Music

Music 可以表达：

```text
Song
Album
Artist
```

其中：

```text
Song
 └── part_of → Album

Song
 └── performed_by → Artist
```

如果当前 Entity + Relation 已经可以表达，就不要新增复杂模型。

---

# 17. Book / Novel

同样：

```text
Work
type = book
```

或者：

```text
Work
type = novel
```

可以记录：

```text
planned
in_progress
completed
revisit
```

并通过：

```text
Memory
Timeline
Collection
Asset
ExternalReference
```

扩展。

---

# 18. Person / Character

建议允许：

```text
Person
```

作为通用 Entity 类型。

例如：

```text
久保带人
```

```text
黑崎一护
```

```text
某个游戏角色
```

但：

> Character 是否单独建模可以暂缓。

当前完全可以：

```text
Entity
type = person
subtype = character
```

或者通过 metadata 表达。

---

# 19. Relation

Personal Records 对 Relation 的需求会非常大。

建议确保 Relation 至少能够表达：

```text
created_by
created
performed_by
developed_by
published_by
adaptation_of
sequel_of
prequel_of
part_of
related_to
inspired_by
features
```

但不要一次性强制增加几十种 relation。

如果当前 Relation 已经是通用字符串/枚举系统：

> 优先扩充可扩展性，而不是堆硬编码。

---

# 20. Memory

Memory 是 Personal Record Domain 的核心。

它不是：

> “另一种 Note。”

而应该代表：

> **用户关于某个对象、某个时间、某件事情的个人记忆。**

例如：

```text
Memory
├── content
├── created_at
├── event_time
├── related_entities
├── related_assets
└── related_collection
```

具体字段以当前 Memory 模型为准。

---

# 21. Life Record

除了娱乐内容，Looma 还应该能记录：

```text
人生事件
项目经历
工作经历
学习经历
旅行
重要事件
个人阶段
成就
```

例如：

```text
Timeline
2026-08
└── AI Role 稳定下来

Memory
└── 对这一阶段的总结

Assets
├── Screenshot
├── GitHub Project
└── Documents
```

---

# 22. Project

Project 可以作为通用 Entity 类型。

例如：

```text
Project
├── Looma
├── AI4PPAP
├── 蛟龙16 Pro Control Panel
└── Other Projects
```

一个 Project 可以连接：

```text
Assets
Memories
Timeline
ExternalReferences
People
Organizations
Technology
```

---

# 23. Personal Timeline

Timeline 应该逐渐成为：

> **个人数字人生时间轴。**

不仅仅是文件时间。

例如：

```text
2026
│
├── 01
│
├── 03
│
├── 08
│   └── AI Role
│
├── 09
│   ├── BLEACH
│   ├── Looma
│   └── Personal Project
│
└── 10
    └── Looma Architecture
```

---

# 24. Timeline Event

如果当前 Timeline 还比较弱，建议确保它可以表达：

```text
event_time
title
description
related_entity
related_asset
related_memory
```

但不要急着做复杂 Calendar 系统。

Looma Timeline 的重点是：

> **记录“我的人生和数字活动发生了什么”。**

---

# 25. Entertainment Dashboard

当前不要求开发复杂 UI。

但 Domain API 应该能够支持未来查询：

```text
正在看的番
看完的番
想看的番
最近看的作品
最近记录的游戏
最常访问的作品
某一年看过什么
```

这些应该通过：

```text
Search
Collection
Timeline
Entity
Memory
```

组合得到。

不要为每一个 Dashboard 创建独立数据库。

---

# 26. Personal Record Query

建议 Core 层未来能够支持类似语义：

```text
find works by type
find works by status
find works by collection
find works by timeline
find memories related to work
find assets related to work
find external references
```

例如：

```text
“我 2026 年看过哪些番？”
```

最终可以组合：

```text
Timeline
+
Work.type = anime
+
event_time.year = 2026
```

---

# 27. MCP 对 Personal Records 的支持

不需要为每一种类型创建：

```text
create_anime
create_game
create_movie
create_book
```

不要这样扩张 MCP Tool 数量。

继续使用通用能力：

```text
create_entity
update_entity
create_relation
create_memory
create_collection
search
```

例如：

```text
create_entity(
    type="work",
    subtype="anime",
    name="BLEACH"
)
```

或者根据当前 Entity API 采用更合适的 schema。

---

# 28. MCP Resource

未来 AI 可以读取：

```text
looma://entity/bleach
```

得到：

```text
Work
├── metadata
├── status
├── relations
├── memories
├── timeline
├── collections
├── assets
└── external references
```

这样 AI 不需要理解 SQLite。

---

# 29. AI 使用场景

这个扩展包最终希望支持：

```text
“我最近看了什么番？”

“我有哪些还没看完的作品？”

“帮我找出 2026 年的游戏记录。”

“我什么时候开始看 BLEACH 的？”

“我之前对这个作品记过什么？”

“这个 GitHub 项目和哪些资产有关？”

“把我今年的个人项目整理成时间线。”
```

这些问题的答案应该来自 Looma 的结构化 Domain。

---

# 30. Import / Manual Entry

v0.1 / v0.2 不要求自动爬取所有动漫网站。

优先支持：

```text
Manual Entry
CLI
MCP
Import
```

未来可以增加：

```text
AniList
Bangumi
TMDB
IGDB
Steam
Goodreads
MusicBrainz
```

但这些全部应该是：

```text
Importer / External Provider
```

而不是 Core 的一部分。

---

# 31. External Provider 原则

例如未来：

```text
Bangumi Importer
AniList Importer
Steam Importer
TMDB Importer
```

它们负责：

```text
External API
    ↓
Normalize
    ↓
Looma Entity
```

不要让：

```text
Core
```

直接依赖这些网站。

---

# 32. Import 数据模型

未来导入时：

```text
External Data
      ↓
Normalization
      ↓
Entity / Work
      ↓
ExternalReference
```

例如：

```text
Bangumi
    ↓
Work
    ↓
ExternalReference
    └── Bangumi URL
```

这样即使以后更换数据源：

```text
Bangumi
→ AniList
```

Looma 内部数据仍然存在。

---

# 33. Duplicate / Identity

这是未来非常重要的问题。

例如：

```text
BLEACH
```

可能来自：

```text
Bangumi
AniList
Wikipedia
User Manual Entry
```

不能因此创建：

```text
BLEACH #1
BLEACH #2
BLEACH #3
BLEACH #4
```

应该未来支持：

```text
External Identity
```

例如：

```text
Work
└── ExternalReference
    ├── bangumi_id
    ├── anilist_id
    └── ...
```

本扩展包当前可以先设计，不要求完整自动去重。

---

# 34. Domain Schema 原则

必须避免：

```text
AnimeTable
GameTable
MovieTable
BookTable
LifeTable
ProjectTable
```

无限膨胀。

优先：

```text
Entity
Work
Memory
Relation
Timeline
Collection
Asset
ExternalReference
```

通过：

```text
type
subtype
metadata
relation
```

实现领域扩展。

---

# 35. Recommended Domain Types

建议当前至少支持以下概念：

```text
Entity
├── Work
├── Person
├── Organization
├── Project
├── Place
└── Topic
```

Work：

```text
Work
├── Anime
├── Manga
├── Game
├── Movie
├── TV Series
├── Music
├── Album
├── Book
└── Novel
```

---

# 36. 最小实现范围

本扩展包不是要求全部功能一次实现。

优先：

```text
P0
├── Work
├── Work Type
├── Status
├── Personal Memory ↔ Work
├── Timeline ↔ Work
├── Collection ↔ Work
├── Asset ↔ Work
└── ExternalReference ↔ Work
```

---

# 37. P1

```text
P1
├── Person
├── Project
├── Character subtype
├── Relation vocabulary
└── Personal Record queries
```

---

# 38. P2

```text
P2
├── External Importer
├── AniList
├── Bangumi
├── TMDB
├── Steam
├── Goodreads
└── Music providers
```

---

# 39. P3

暂不实现：

```text
Episode-level watch tracking
Chapter-level reading tracking
Automatic recommendation engine
AI-generated ratings
Automatic web crawling
Complex media database
Social features
Public profile
```

---

# 40. 与当前 Architecture Hardening 的关系

必须保持：

```text
Architecture
      ↓
Domain
      ↓
Adapters
```

而不是：

```text
Anime
 ↓
Special database
 ↓
Special API
 ↓
Special MCP
```

正确结构：

```text
                  LoomaCore
                      │
          ┌───────────┼───────────┐
          ↓           ↓           ↓
        Asset       Entity      Memory
                      │
                      ↓
                    Work
                      │
       ┌──────────────┼──────────────┐
       ↓              ↓              ↓
     Anime           Game           Book
       │              │              │
       └──────────────┼──────────────┘
                      ↓
                  Relations
                      │
          ┌───────────┼───────────┐
          ↓           ↓           ↓
       Timeline   Collection    Assets
                                  │
                                  ↓
                        ExternalReference
```

---

# 41. 验收标准

完成后必须可以至少表达以下场景：

### 场景 A — 番剧

```text
创建 BLEACH
type = anime

加入“我喜欢的番”

记录观看时间

添加 Memory

添加截图 Asset

添加外部链接
```

---

### 场景 B — 游戏

```text
创建游戏
type = game

status = in_progress

关联截图

关联 Memory

加入 Collection
```

---

### 场景 C — 项目

```text
创建 Project

关联：
├── Assets
├── Memories
├── Timeline
└── ExternalReferences
```

---

### 场景 D — 人生记录

```text
创建 Timeline Event

关联：
├── Project
├── Work
├── Asset
└── Memory
```

---

# 42. MCP 验收

通过 MCP 至少能够：

```text
create Work
update Work
search Work
create Memory
relate Work ↔ Memory
relate Work ↔ Asset
create Collection
add Work to Collection
read Work Resource
```

不要增加大量专用 MCP Tool。

---

# 43. CLI 验收

CLI 至少应该能够通过通用能力完成：

```text
create
search
show
relate
memory
collection
timeline
```

具体命令格式根据当前 CLI 风格设计。

---

# 44. GUI 验收

本扩展包当前不要求设计大型新 UI。

只要求：

> 当前 GUI 如果已经存在 Entity / Memory / Collection / Timeline 等入口，不应阻碍 Work 类型的使用。

后续可以再设计：

```text
Entertainment View
Project View
Life Timeline View
```

---

# 45. 数据兼容

必须保证：

```text
旧 Entity
旧 Asset
旧 Memory
旧 Relation
旧 Collection
```

继续可用。

如果需要 Schema Migration：

```text
migration
```

优先。

禁止：

```text
reset database
delete existing records
```

---

# 46. 最终目标

完成本扩展包后，Looma 应该能够同时表达：

```text
我的文件
我的作品
我的游戏
我的番剧
我的书
我的音乐
我的项目
我的人物
我的记忆
我的人生事件
我的时间线
我的收藏
我的外部数字足迹
```

并且它们最终都可以连接成：

```text
                         My Digital Life
                                │
        ┌───────────────────────┼───────────────────────┐
        ↓                       ↓                       ↓
   Entertainment             Projects                  Life
        │                       │                       │
   ┌────┼────┐             ┌────┼────┐             ┌───┼───┐
   ↓    ↓    ↓             ↓    ↓    ↓             ↓   ↓   ↓
 Anime Game Book         Looma AI Project        Events Memories
   │    │    │             │    │    │             │
   └────┼────┘             └────┼────┘             │
        │                       │                  │
        └───────────────┬───────┴──────────────────┘
                        ↓
                       Core
                        │
               ┌────────┼────────┐
               ↓        ↓        ↓
             Assets  Relations Timeline
               │        │        │
               └────────┼────────┘
                        ↓
               External References
```

---

# 47. 最重要的设计原则

> **Looma 不是用来管理“动漫”的。**

> **Looma 是用来管理“我与数字世界之间发生过什么”的。**

动漫、游戏、电影、音乐、书籍、项目，只是不同类型的数字对象。

最终应该能够形成：

```text
Object
+
Relationship
+
Memory
+
Time
+
Asset
+
Collection
+
External Reference
```

这才是 Personal Digital OS 的核心。

---

# 48. 给实现 Agent 的最终指令

请基于当前已经完成的：

```text
Looma v0.2 Architecture Hardening
```

实现本扩展包。

执行前：

```text
1. Inspect current Domain Model
2. Inspect Entity / Asset / Memory / Relation / Collection / Timeline
3. Determine which capabilities already exist
4. Reuse existing architecture
5. Do not duplicate existing concepts
```

然后优先实现：

```text
P0
Work
Work Types
Status
Work ↔ Memory
Work ↔ Timeline
Work ↔ Collection
Work ↔ Asset
Work ↔ ExternalReference
```

其次：

```text
P1
Person
Project
Relation vocabulary
Personal Record queries
```

最后：

```text
P2
External Importer boundary
```

暂不实现 P3。

每完成一个阶段，输出：

```text
Implemented
Reused
Changed
Migration
Tests
Remaining Gap
```

**不要因为文档中出现了 Anime / Game / Book，就分别建立独立数据库系统。**

优先保持：

```text
Entity
Work
Relation
Memory
Timeline
Collection
Asset
ExternalReference
```

这套统一模型。

最终验收标准：

> **我可以把“我看过什么、玩过什么、做过什么、经历过什么、留下了什么文件、写过什么记忆、这些东西之间有什么关系”统一放进 Looma，并让 CLI / MCP / GUI 使用同一套 Core。**
