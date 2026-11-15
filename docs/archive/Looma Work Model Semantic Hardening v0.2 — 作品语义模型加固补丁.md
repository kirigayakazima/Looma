# Looma Work Model Semantic Hardening v0.2
## 作品语义模型加固补丁

> **目标**：在 Generic Work Type Expansion v0.1 已完成的基础上，对 Work 的「身份、类型元数据、生命周期、进度、外部引用、作品关系」进行一次语义层加固。
>
> **核心原则**：不新增 Anime/Manga/Game/Book/Movie 专属数据库表，不建立专属 Service，不破坏现有 `LoomaCore → GUI / CLI / MCP` 统一架构。
>
> **本补丁定位**：Architecture / Domain Semantics Hardening
>
> **重要**：本补丁不是新功能大版本，不要求重新设计数据库，不要求拆分 Work 模型。

---

# 1. 背景

当前 Generic Work Type Expansion v0.1 已经实现：

```text
Work
├── Game
├── Anime
├── Manga
├── Book
├── Movie
├── TvSeries
├── Music
└── Custom
```

并统一复用：

```text
entities
assets
relations
collections
memories
external_references
WorkProgress
LoomaCore
```

当前架构方向正确。

但继续扩展 Anime / Manga / Book / Movie 等功能之前，需要先解决一个潜在问题：

> `type_metadata` 不能逐渐演化成一个“隐藏的 AnimeTable / MangaTable”。

因此本补丁必须明确 Work 内部的语义边界。

---

# 2. 最终 Work 语义骨架

以后所有 Work 类型必须遵守以下模型：

```text
Work
│
├── Identity
│   ├── id
│   ├── title
│   ├── aliases
│   └── work_type
│
├── Metadata
│   └── type_metadata
│
├── Lifecycle
│   └── status
│
├── Progress
│   ├── position
│   ├── total_positions
│   ├── position_type
│   └── unit
│
└── Connections
    ├── Assets
    ├── ExternalReferences
    ├── Memories
    ├── Collections
    └── Relations
```

这五层必须保持概念独立。

---

# 3. P0：Identity 与 Metadata 必须分离

## 3.1 Identity

Identity 表示：

> “这个东西是谁？”

至少包含：

```text
id
title
work_type
aliases
```

其中：

```text
id
```

是 Looma 内部稳定身份。

---

## 3.2 title

`title` 是用户主要看到的作品名称。

例如：

```text
BLEACH
死神
BLEACH 千年血战篇
```

不要把别名全部塞进 title。

---

## 3.3 aliases

如果当前 Work 模型没有 aliases 字段：

### 优先方案

增加通用 `aliases` 能力，但必须保持为 Work 通用属性。

不能创建：

```text
AnimeAliases
MangaAliases
GameAliases
```

可以使用：

```text
Vec<String>
```

或现有 JSON metadata 中的通用字段。

---

## 3.4 Alias 的用途

主要解决：

```text
BLEACH
死神
ブリーチ
Bleach
```

以及：

```text
NieR:Automata
NieR Automata
NierAutomata
```

之间的身份匹配问题。

它主要服务于：

```text
搜索
去重
Candidate Match
External Reference Match
Scanner Match
```

---

# 4. P0：TypeSpecificMetadata 只负责“作品描述”

当前：

```rust
TypeSpecificMetadata
```

设计继续保留。

允许：

```text
Game
Anime
Manga
Book
Movie
TvSeries
Music
Custom
```

但是必须增加明确红线：

## Metadata 可以保存

例如 Anime：

```json
{
  "kind": "anime",
  "studio": "...",
  "season": "...",
  "year": 2026
}
```

Manga：

```json
{
  "kind": "manga",
  "author": "...",
  "artist": "...",
  "publisher": "..."
}
```

Book：

```json
{
  "kind": "book",
  "author": "...",
  "isbn": "..."
}
```

Game：

```json
{
  "kind": "game",
  "developer": "...",
  "publisher": "...",
  "platform": [...]
}
```

---

# 5. P0：Metadata 严禁保存 Lifecycle / Connection 数据

禁止将以下内容塞进 `type_metadata`：

```text
watch_status
reading_status
playing_status

current_episode
current_chapter
current_page

local_paths
asset_ids

external_urls
references

memories
notes

collection_ids
```

也禁止：

```json
{
  "kind": "anime",
  "current_episode": 13,
  "watch_status": "watching"
}
```

因为这些属于 Work 的通用生命周期或连接层。

---

# 6. P0：Lifecycle 与 Progress 必须独立

统一定义：

```text
Lifecycle Status
+
Progress
```

而不是将二者混合。

---

## 6.1 Lifecycle Status

建议继续统一使用通用状态：

```text
planned
in_progress
completed
paused
dropped
archived
```

如果当前系统已经存在其他状态名称：

> 优先保持现有兼容值，不进行破坏性重命名。

---

## 6.2 Progress

继续复用：

```text
WorkProgress
```

结构保持：

```text
position
total_positions
position_type
unit
```

其中：

```text
total_positions: Option<f64>
```

必须继续支持：

```text
Some(...)
None
```

---

# 7. Progress 使用规范

统一允许：

### Anime

```text
position = 13
total = 24
unit = episode
```

### Manga

```text
position = 143
total = None
unit = chapter
```

### Book

```text
position = 237
total = 500
unit = page
```

### Movie

```text
position = 68
total = 120
unit = minute
```

### Game

```text
position = 65
total = 100
unit = percent
```

但不得创建：

```text
AnimeProgress
MangaProgress
GameProgress
BookProgress
```

---

# 8. P0：Status 与 Progress 的语义关系

允许出现：

```text
status = in_progress
progress = 13 / 24
```

也允许：

```text
status = paused
progress = 13 / 24
```

也允许：

```text
status = completed
progress = 24 / 24
```

但：

> 修改 progress 不应该自动偷偷修改 status，除非当前系统已有明确且稳定的业务规则。

例如：

```text
update_work_progress()
```

默认只更新 Progress。

```text
update_work_status()
```

默认只更新 Lifecycle。

这样避免：

```text
用户改到第 13 集
→ 系统偷偷把状态改成 watching
```

造成不可预测行为。

---

# 9. P0：ExternalReference 必须成为独立 Connection

一个 Work 可以拥有：

```text
0..N ExternalReference
```

例如：

```text
BLEACH
│
├── Bangumi
├── Bilibili
├── Official Website
└── Wiki
```

游戏：

```text
NieR:Automata
│
├── Steam
├── Official Website
└── Wiki
```

---

# 10. ExternalReference 建议语义

保持当前 `external_references` 表，不新增表。

至少能够表达：

```text
provider
url
title / label
is_primary
```

如果现有模型不方便增加字段：

> 优先复用现有 JSON / metadata 扩展能力，不要新建专用表。

---

## 10.1 primary reference

允许一个 Work 指定一个主要入口：

```text
is_primary = true
```

例如：

```text
Anime
├── Bangumi       primary
├── Bilibili
└── Official
```

UI 可以显示：

```text
🔗 主入口
[打开 Bangumi]
```

---

# 11. P1：作品关系必须支持“作品宇宙”

这是本次补丁非常重要的一项。

Work 本身不要试图容纳：

```text
原作
动画
漫画
游戏
第一季
第二季
剧场版
续作
```

这些应该继续通过：

```text
relations
```

表达。

---

# 12. 推荐支持的 Relation 类型

保持 Relation 为通用机制。

建议确认以下 relation type 可以表达：

```text
adaptation_of
sequel_of
prequel_of
part_of
spin_off_of
remake_of
based_on
related_to
```

不要求一次新增全部类型。

如果现有系统已经支持类似语义：

> 优先复用现有名称，不创建重复关系类型。

---

# 13. 典型作品结构

例如：

```text
BLEACH Manga
        │
        │ adaptation_of
        ▼
BLEACH Anime
        │
        ├── sequel_of
        │      ▼
        │   TYBW Season 1
        │
        ├── sequel_of
        │      ▼
        │   TYBW Season 2
        │
        └── sequel_of
               ▼
            TYBW Season 3
```

游戏：

```text
NieR
 │
 ├── related_to → NieR:Automata
 │
 └── adaptation / related
```

这样以后 Looma 才能真正形成：

> “我的作品宇宙”

而不是大量孤立卡片。

---

# 14. P1：不要过早实现复杂 Series 模型

本补丁明确禁止：

```text
AnimeSeriesTable
SeasonTable
FranchiseTable
UniverseTable
```

也不要现在创建：

```text
AnimeSeasonService
FranchiseService
```

当前只需要：

```text
Work
+
Relation
```

即可。

以后如果真正出现无法用 Relation 表达的需求，再单独进行架构评估。

---

# 15. P1：Work 与 Asset 继续完全解耦

必须允许：

### 只有 Work

```text
Anime
└── no local asset
```

### Work + Local Asset

```text
Game
└── E:/Games/xxx
```

### Work + External Reference

```text
Anime
└── Bangumi URL
```

### Work + Memory

```text
Anime
└── 观后感
```

### Work + Local + External + Memory + Progress

全部同时存在。

---

# 16. P1：本地资源不是作品身份

必须继续坚持：

```text
Work ID ≠ Asset ID
```

例如：

```text
BLEACH
│
├── Asset A
│   └── E:/Anime/BLEACH
│
├── Asset B
│   └── F:/Backup/BLEACH
│
└── Asset C
    └── E:/Pictures/BLEACH.jpg
```

删除某个 Asset：

> 不得删除 Work。

目录丢失：

> 不得删除 Work。

重新定位：

> 不得改变 Work ID。

---

# 17. P1：Relocate 必须保持 Identity

继续保持：

```text
old Work ID
       │
       ├── old Asset relation removed
       │
       └── new Asset relation added
```

不能：

```text
delete old Work
create new Work
```

必须保证：

```text
Work ID unchanged
```

并且：

```text
Memories
Collections
ExternalReferences
Relations
```

继续挂在原 Work 上。

---

# 18. P1：Work Summary / WorkProfile 语义固定

当前：

```rust
pub type WorkProfile = WorkSummary;
```

可以继续使用。

但要求明确：

> `WorkProfile` 是一个聚合读取模型，不是新的持久化实体。

它可以包含：

```text
Work
├── Metadata
├── Progress
├── Assets
├── ExternalReferences
├── Memories
├── Collections
└── Relations
```

但不能创建：

```text
work_profiles
```

数据库表。

---

# 19. GUI：不要为每种 Work 创建完全独立的数据模型

推荐统一：

```text
WorksView
```

然后根据：

```text
work_type
```

决定：

```text
显示哪些字段
```

例如：

```text
Anime
├── title
├── studio
├── season
├── progress
└── references

Manga
├── title
├── author
├── artist
├── progress
└── references

Game
├── title
├── developer
├── publisher
├── local paths
└── progress
```

但底层仍然是：

```text
WorkSummary
```

---

# 20. GUI：状态与进度必须统一显示

推荐统一卡片信息：

```text
┌──────────────────────────┐
│ BLEACH                   │
│ Anime                    │
│                          │
│ ● 观看中                 │
│ 第 13 / 24 集            │
│                          │
│ 🔗 2 个在线入口           │
│ 📁 1 个本地资源           │
│ 📝 3 篇手记               │
└──────────────────────────┘
```

Manga：

```text
┌──────────────────────────┐
│ 某漫画                   │
│ Manga                    │
│                          │
│ ● 阅读中                 │
│ 第 143 话 / 连载中       │
│                          │
│ 🔗 1 个在线入口           │
│ 📝 2 篇手记               │
└──────────────────────────┘
```

不要让 AnimeView / MangaView 各自重新实现整套状态逻辑。

---

# 21. CLI：保持统一 Work 命令

继续使用：

```text
looma work list
looma work create
looma work show
looma work status
looma work progress
looma work link-dir
looma work relocate-dir
```

不要新增：

```text
looma anime ...
looma manga ...
looma game ...
```

除非未来有明确的纯快捷命令需求。

---

# 22. MCP：保持统一 Work 工具

继续使用：

```text
create_work
update_work_status
update_work_progress
link_work_directory
relocate_work_directory
```

AI 查询：

```text
looma://work/{id}
```

继续返回聚合 Work Profile。

不要创建：

```text
create_anime
create_manga
create_game
```

作为新的平行 Domain API。

---

# 23. P1：Search / Dedup 必须考虑 aliases

未来 Scanner / Candidate / External Reference 匹配时：

```text
title
+
aliases
```

共同参与匹配。

例如：

```text
BLEACH
```

应该可以匹配：

```text
死神
ブリーチ
Bleach
```

但：

> 本补丁只要求模型能够表达 aliases，不要求现在实现复杂 AI 模糊匹配。

简单规范化匹配即可。

---

# 24. P1：不要在本补丁引入“自动联网识别”

当前 Looma 是 Local-First。

因此本补丁：

**禁止自动：**

```text
扫描本地文件
→ 自动请求 Bangumi
→ 自动创建作品
→ 自动修改 metadata
```

ExternalReference 仍然应该是：

```text
用户显式添加
```

或者未来单独设计：

```text
Suggestion
```

机制。

---

# 25. Backward Compatibility

必须保证现有 Work 数据继续可读取。

特别是：

```text
没有 type_metadata
```

的旧数据。

读取时必须正常：

```text
type_metadata = None
```

或使用当前系统已有默认行为。

不得因为增加：

```text
aliases
status
metadata
references
```

导致旧 SQLite 数据无法读取。

---

# 26. Migration 红线

本补丁优先：

```text
NO NEW TABLE
NO NEW SERVICE
NO PARALLEL DOMAIN
```

允许：

```text
现有 JSON 字段扩展
现有 Model 扩展
现有 Relation type 扩展
现有 ExternalReference 扩展
```

只有当当前 Schema 确实无法表达需求时，才允许新增 Migration。

如果需要 Migration：

> 必须先停止实现，并在审计报告中明确说明原因。

---

# 27. 必须新增的测试

至少新增以下测试。

## Test A：Metadata 与 Progress 隔离

验证：

```text
Anime.type_metadata
```

不会携带：

```text
current_episode
watch_status
asset_ids
```

---

## Test B：Status 与 Progress 独立

验证：

```text
update_work_progress()
```

不会意外修改：

```text
status
```

除非当前已有明确规则。

---

## Test C：无 Asset Work

```text
Anime
├── Asset = 0
├── Reference >= 1
├── Progress exists
└── Memory exists
```

必须正常。

---

## Test D：Multi Reference

验证：

```text
Work
├── Bangumi
├── Official
└── Bilibili
```

可以同时存在。

---

## Test E：Relation Identity

验证：

```text
A adaptation_of B
```

不会改变：

```text
A.id
B.id
```

---

## Test F：Relocate Identity

继续验证：

```text
Work ID before relocate
==
Work ID after relocate
```

同时：

```text
Memory
Collection
ExternalReference
```

仍然保持关联。

---

## Test G：Alias Matching

验证：

```text
BLEACH
死神
ブリーチ
```

可以作为同一 Work 的 alias。

---

## Test H：Restart Persistence

重启 SQLite 后验证：

```text
Work
Metadata
Status
Progress
Assets
References
Memories
Collections
Relations
Aliases
```

全部保持。

---

# 28. Acceptance Checklist

本补丁完成后，必须达到：

| 项目 | 要求 |
|---|---|
| Work Identity | PASS |
| Alias | PASS |
| Type Metadata | PASS |
| Lifecycle Status | PASS |
| Progress | PASS |
| Metadata / Lifecycle 分离 | PASS |
| Asset / Work 分离 | PASS |
| ExternalReference | PASS |
| Multi Reference | PASS |
| Relation | PASS |
| Adaptation / Sequel 关系 | PASS |
| Relocate Identity | PASS |
| Memory 保留 | PASS |
| Collection 保留 | PASS |
| CLI | PASS |
| MCP | PASS |
| GUI | PASS |
| Restart Persistence | PASS |
| Backward Compatibility | PASS |
| No Game/Anime/Manga Table | PASS |
| No Game/Anime/Manga Service | PASS |

---

# 29. 最终架构红线

以后继续开发 Looma Work 系统时，必须遵守：

```text
❌ AnimeTable
❌ MangaTable
❌ GameTable
❌ BookTable

❌ AnimeService
❌ MangaService
❌ GameService

❌ AnimeProgress
❌ MangaProgress
❌ GameProgress

❌ AnimeAsset
❌ MangaAsset
❌ GameAsset

❌ AnimeView 私有数据库逻辑
❌ MangaView 私有数据库逻辑
❌ GameView 私有数据库逻辑

                    ↓

              LoomaCore
                  │
        ┌─────────┼─────────┐
        ↓         ↓         ↓
       GUI       CLI       MCP
                  │
                  ↓
                SQLite
```

所有 Work 类型只是：

```text
Work
+ work_type
+ type_metadata
```

而不是新的领域系统。

---

# 30. 本补丁完成后的目标模型

最终希望 Looma 达到：

```text
                         ┌───────────────┐
                         │     Work      │
                         └───────┬───────┘
                                 │
             ┌───────────────────┼───────────────────┐
             │                   │                   │
             ▼                   ▼                   ▼
         Identity            Metadata            Lifecycle
             │                   │                   │
       title / alias       type-specific          status
       work_type            information
                                 │
                                 ▼
                              Progress
                                 │
             ┌───────────────────┼───────────────────┐
             │                   │                   │
             ▼                   ▼                   ▼
           Assets            References           Memories
             │                   │                   │
             └───────────────────┼───────────────────┘
                                 │
                                 ▼
                            Collections
                                 │
                                 ▼
                             Relations
                                 │
                    ┌────────────┼────────────┐
                    ▼            ▼            ▼
               adaptation     sequel       related
```

这套结构应该成为 Looma 后续：

```text
Anime
Manga
Game
Book
Movie
TV Series
Music
Novel
Custom
```

全部扩展的共同基础。

---

# 31. 实施策略

本次不要大改。

推荐按照：

```text
Phase 1
  Identity / aliases
       ↓
Phase 2
  Metadata boundary
       ↓
Phase 3
  Status / Progress separation
       ↓
Phase 4
  ExternalReference semantics
       ↓
Phase 5
  Relation semantics
       ↓
Phase 6
  GUI / CLI / MCP consistency
       ↓
Phase 7
  Regression tests
```

逐项实施。

如果现有实现已经满足某项：

> 不要为了“看起来完成”而重构。

优先添加：

```text
tests
validation
documentation
```

---

# 32. 最终交付要求

完成后不要只报告：

```text
15/15 PASS
```

必须额外报告：

```text
1. 是否修改 SQLite Schema
2. 是否新增 Migration
3. 是否新增专属 Service
4. 是否新增专属 Table
5. aliases 如何实现
6. type_metadata 与 lifecycle 的边界
7. status 与 progress 是否独立
8. ExternalReference 是否支持多个
9. Relation 支持哪些作品关系
10. relocate 是否保持 Work ID
11. GUI / CLI / MCP 是否仍统一走 LoomaCore
12. Backward Compatibility 测试结果
```

最终审计报告应明确给出：

```text
ARCHITECTURE: PASS / PARTIAL / FAIL
SEMANTICS: PASS / PARTIAL / FAIL
PERSISTENCE: PASS / PARTIAL / FAIL
BACKWARD_COMPATIBILITY: PASS / PARTIAL / FAIL
GUI/CLI/MCP CONSISTENCY: PASS / PARTIAL / FAIL
```

只有全部达到 PASS，才允许将：

```text
Work Model Semantic Hardening v0.2
```

标记为：

```text
FROZEN
```

---

# 33. 最重要的一句话

**Looma 不应该拥有“Anime 系统、Manga 系统、Game 系统”。**

Looma 应该拥有：

> **一个足够稳定的 Work 系统，而 Anime / Manga / Game / Book 只是这个 Work 系统的不同语义类型。**

本补丁的目的，就是在继续扩展功能之前，把这条边界彻底固定下来。