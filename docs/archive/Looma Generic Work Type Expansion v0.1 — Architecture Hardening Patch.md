# Looma Generic Work Type Expansion v0.1
## Architecture Hardening & Media Work Model Patch

> **目标阶段**：Game Library v0.1 → Generic Work Type Expansion v0.1  
> **适用对象**：Looma Core / Database / CLI / MCP / Desktop GUI  
> **前置状态**：Game Library v0.1 已完成 P0/P1 修复并通过回归测试  
> **核心原则**：不推翻现有 Game Library，不创建 Anime/Manga/Book 专属平行数据库模型

---

# 0. 本补丁的核心目标

Game Library v0.1 已经证明：

```text
Work
  ↓
LoomaCore
  ↓
Database
  ↓
Asset / Relation / Collection / Memory / ExternalReference
  ↓
GUI / CLI / MCP
```

这套架构可以承载一个完整的“作品档案”。

下一阶段不要继续：

```text
Game
 ├── GameTable
 ├── GameProgress
 ├── GameLinks
 └── GameLibraryService
```

然后 Anime 再复制：

```text
AnimeTable
AnimeProgress
AnimeLinks
AnimeService
```

这是明确禁止的。

正确方向必须是：

```text
                 Work
                  │
      ┌───────────┼───────────┐
      ↓           ↓           ↓
    Game        Anime        Manga
      ↓           ↓           ↓
    Book        Movie       Project
```

所有类型共享：

```text
Work
WorkMetadata
WorkProgress
Asset
Relation
Collection
Memory
ExternalReference
```

类型差异只存在于：

```text
WorkType
+
type-specific metadata
+
GUI presentation
```

---

# 1. 重要架构修正：不要把 Game Library 的成功实现误认为架构已经完全冻结

当前 Audit 中存在：

> Game Library v0.1 正式冻结

这句话可以保留，但必须解释为：

```text
Game-specific feature implementation = FROZEN
```

而不是：

```text
Work architecture = FROZEN
```

因为接下来 Anime / Manga / Book 等类型会暴露现有 Work 模型中的通用能力边界。

因此：

## 禁止

为了 Anime / Manga：

```text
AnimeTable
MangaTable
BookTable
MovieTable
```

## 允许

扩展：

```text
WorkType
WorkMetadata
WorkProgress
ExternalReference
```

并增强通用 Core Facade。

---

# 2. Work Type 扩展

当前：

```text
Game
```

扩展为至少：

```text
Game
Anime
Manga
Book
Movie
TVSeries
Project
```

注意：

不要为了“完整”一次性加入几十种类型。

第一阶段只需要确保模型能够承载：

```text
Game
Anime
Manga
Book
Movie
```

即可。

---

# 3. Work 与 Asset 必须保持严格分离

这是本阶段最重要的概念之一。

例如：

```text
BLEACH
```

本身是：

```text
Work(type=anime)
```

它可以没有任何本地 Asset。

例如：

```text
BLEACH
├── Work
├── ExternalReference
│   ├── Bangumi
│   ├── Official Website
│   └── Wikipedia
├── Progress
│   └── Episode 42
└── Memory
    └── 我的观后感
```

也完全成立。

---

# 4. 一个 Work 不要求必须拥有本地资源

必须明确支持：

```text
Work
 ├── no local Asset
 ├── ExternalReference
 ├── Progress
 ├── Memory
 └── Collection
```

例如：

```text
某部番剧
```

用户只是：

```text
看到第 8 集
```

但本地没有下载任何内容。

Looma 仍然应该能够完整记录。

---

# 5. Work 可以同时拥有本地资源与外部资源

例如：

```text
BLEACH 千年血战篇
```

可能是：

```text
Work
│
├── Asset
│   ├── 本地视频目录
│   ├── 海报
│   └── 截图
│
├── ExternalReference
│   ├── Bangumi
│   ├── 官方网站
│   └── Wikipedia
│
├── Progress
│   └── Episode 17
│
├── Memory
│   └── 观后感
│
└── Collection
    └── 我喜欢的动画
```

这是 Looma 应该重点支持的标准形态。

---

# 6. Progress 必须继续保持 Generic

当前 Game 已经复用了：

```text
WorkProgress
```

不要为 Anime / Manga 增加：

```text
AnimeProgress
MangaProgress
BookProgress
```

而是继续使用：

```text
WorkProgress
```

但是需要检查它是否能够表达：

## Anime

```text
position = 17
position_type = episode
total_positions = 26
unit = episode
```

## Manga

```text
position = 82
position_type = chapter
total_positions = 120
unit = chapter
```

## Book

```text
position = 143
position_type = page
total_positions = 500
unit = page
```

## Movie

```text
position = 70
position_type = minute
total_positions = 125
unit = minute
```

如果现有 `WorkProgress` 已经可以表达，则不要修改模型。

如果不能表达：

> 优先增强通用字段，而不是创建新的类型专属 Progress。

---

# 7. Progress 必须允许“不知道总数”

例如：

```text
某连载漫画
```

可能只有：

```text
position = 142
position_type = chapter
total_positions = null
```

不能强迫：

```text
total_positions = 9999
```

否则会破坏现实世界中的：

```text
连载漫画
连载动画
未完结作品
```

---

# 8. Progress Status 与 Work Type 解耦

状态应该保持通用：

```text
planned
in_progress
completed
paused
dropped
```

不要：

```text
anime_watching
manga_reading
game_playing
book_reading
```

因为这些其实都是：

```text
Work.status
+
Work.progress
```

的组合。

---

# 9. ExternalReference 继续作为统一外链层

目前 Game 已经支持：

```text
Steam
Website
Bangumi
```

下一阶段必须确保 ExternalReference 可以表达：

```text
provider
url
label
```

例如：

```text
BLEACH
 ├── Bangumi
 ├── MyAnimeList
 ├── Official
 └── Wikipedia
```

Manga：

```text
某漫画
 ├── Bangumi
 ├── 漫画官网
 └── 作者主页
```

Game：

```text
某游戏
 ├── Steam
 ├── 官网
 └── Wikipedia
```

不要为每种作品建立独立 Link 表。

---

# 10. ExternalReference 增强：允许用户区分“来源”

建议至少保留：

```text
provider
url
label
```

例如：

```json
{
  "provider": "bangumi",
  "label": "Bangumi",
  "url": "https://..."
}
```

未来可以增加：

```text
is_primary
```

但当前不强制。

---

# 11. 新增“作品类型语义”，但不要创建专属数据库表

建议 WorkMetadata 能够容纳类型相关信息。

例如 Anime：

```json
{
  "status": "in_progress",
  "cover_asset_id": "...",
  "metadata": {
    "season": "TYBW",
    "year": 2026
  }
}
```

Manga：

```json
{
  "metadata": {
    "author": "...",
    "publisher": "..."
  }
}
```

Book：

```json
{
  "metadata": {
    "author": "...",
    "isbn": "..."
  }
}
```

但是：

> 这些字段必须保持为 Metadata / Properties 层，而不是创建新的实体表。

---

# 12. 建议增加“Work Profile”概念

不要创建新的数据库实体。

只是对现有 WorkSummary 进行语义整理。

例如：

```text
WorkSummary
├── work
├── progress
├── linked_assets
├── linked_memories
├── linked_collections
├── external_references
└── type_metadata
```

这样 GUI / CLI / MCP 都可以使用统一的 Work Profile。

---

# 13. GUI 不应该复制 Game 页面逻辑

当前：

```text
GamesView
```

已经验证了 Work → Core → DB 的正确链路。

下一阶段不要：

```text
GamesView
AnimeView
MangaView
BookView
MovieView
```

全部复制。

建议逐渐抽象：

```text
WorksView
```

或者：

```text
WorkLibraryView
```

然后：

```text
filter:
    game
    anime
    manga
    book
    movie
```

---

# 14. 允许不同 Work Type 使用不同 UI Presentation

统一数据模型不代表 UI 必须完全一样。

例如 Game：

```text
Game
├── 安装位置
├── 启动目录
├── Progress
└── Steam
```

Anime：

```text
Anime
├── 当前集数
├── 总集数
├── Bangumi
└── 本地视频
```

Manga：

```text
Manga
├── 当前章节
├── 总章节
├── 本地漫画目录
└── 在线阅读地址
```

因此：

```text
Core = Generic
GUI = Type-aware
```

这是允许的。

---

# 15. “本地资源”不等于“作品”

必须防止以后出现这种错误：

```text
D:/Anime/BLEACH
```

直接被当成：

```text
Anime
```

正确逻辑：

```text
Directory Asset
      │
      ↓
可能是某个 Work 的 Asset
```

只有用户确认：

```text
BLEACH
    attaches
D:/Anime/BLEACH
```

之后才形成语义关联。

---

# 16. Candidate Detection 也必须保持 Suggestion-only

Game 当前的：

```text
detect_game_candidates
```

是正确方向。

未来可以抽象成：

```text
detect_work_candidates
```

但是：

> 第一阶段不要直接做“自动创建 Anime / Manga”。

候选发现只能返回：

```text
Candidate
```

例如：

```text
Potential Anime:
    D:/Anime/BLEACH

Potential Manga:
    D:/Manga/BLEACH
```

然后：

```text
用户确认
    ↓
创建 Work
    ↓
ensure Asset
    ↓
link Work ↔ Asset
```

---

# 17. Directory Asset 机制必须保持通用

已经完成：

```text
ensure_directory_asset
link_work_directory
```

这套能力不要命名成：

```text
ensure_game_directory
link_game_directory
```

必须保持：

```text
ensure_directory_asset
link_work_directory
```

这样 Anime / Manga / Book / Project 都可以复用。

---

# 18. 一个 Work 可以关联多个 Directory Asset

例如：

```text
BLEACH
├── D:/Anime/BLEACH
├── E:/Archive/BLEACH
└── F:/Backup/BLEACH
```

继续使用：

```text
Relation(attaches)
```

不要增加：

```text
primary_path
secondary_path
archive_path
```

之类的专属字段。

---

# 19. 建议增加 Asset Role，而不是创建新的 Asset 类型

未来可能需要区分：

```text
本地目录
封面
截图
视频
漫画文件
PDF
压缩包
```

但这些应该通过：

```text
AssetKind
+
Relation
```

表达。

例如：

```text
BLEACH
   │
   ├── attaches → D:/Anime/BLEACH
   ├── cover → bleach.jpg
   └── screenshot → screenshot_001.png
```

不要创建：

```text
AnimeAsset
GameAsset
MangaAsset
```

---

# 20. Relation 类型需要检查是否足够

当前已经有：

```text
attaches
```

建议确认 Relation 是否可以逐步表达：

```text
attaches
cover
references
created_by
related_to
```

但：

> 不要为了未来可能存在的关系一次性创建几十种 RelationType。

当前只需要保证：

```text
Work ↔ Asset
Work ↔ Entity
Work ↔ Memory
```

可以持续扩展。

---

# 21. Memory 必须继续保持独立

例如：

```text
BLEACH
```

可以拥有：

```text
Memory:
2026-09-20
“千年血战篇这一集观感……”
```

Memory 不应该塞进：

```text
WorkMetadata
```

也不要：

```text
AnimeReviewTable
GameReviewTable
```

继续使用：

```text
Memory
Relation
```

---

# 22. Collection 继续保持通用

例如：

```text
Collection
├── 我喜欢的动画
├── 待玩的游戏
├── 想看的电影
├── 我的漫画
└── 2026 年作品
```

Collection 不需要知道：

```text
Game
Anime
Manga
```

它只需要知道：

```text
Entity / Work
```

---

# 23. CLI 必须提供统一 Work 能力

现有：

```text
looma work list -t game
looma work create xxx -t game
looma work show xxx
looma work status xxx
looma work progress xxx
looma work link-dir xxx path
```

应该继续适用于：

```text
-t anime
-t manga
-t book
-t movie
```

例如：

```text
looma work create "BLEACH 千年血战篇" -t anime

looma work progress <ID> 17 --unit episode

looma work link-dir <ID> "D:/Anime/BLEACH"

looma work show <ID>
```

---

# 24. MCP 必须继续保持 Generic

不要创建：

```text
create_game
create_anime
create_manga
update_game_progress
update_anime_progress
```

应优先保持：

```text
create_work
update_work_progress
link_work_directory
search_entities
```

通过：

```text
work_type
```

区分。

例如：

```json
{
  "work_type": "anime"
}
```

---

# 25. MCP Resource 必须能够表达完整 Work Profile

未来：

```text
looma://work/{id}
```

应该能够返回：

```json
{
  "work": {},
  "progress": {},
  "assets": [],
  "memories": [],
  "collections": [],
  "external_references": []
}
```

这样 AI 才真正能够理解：

> “我看到哪了？”

而不是只能知道：

> “这个实体叫 BLEACH。”

---

# 26. 重要：不要把“记录观看进度”和“媒体播放状态”混为一谈

例如：

```text
Anime Progress
```

只是：

```text
Episode 17
```

它不代表：

```text
本地播放器正在播放
```

因此当前阶段不要做：

```text
播放器
播放历史
播放时长
自动同步播放器
```

这些属于未来 Media Integration。

---

# 27. 本阶段暂缓功能

明确禁止反重力在本补丁中顺手加入：

```text
OCR
向量数据库
Semantic Search
LLM 自动分类
自动联网搜索
Bangumi API 自动同步
Steam API 自动同步
播放器
漫画阅读器
视频播放器
自动下载
云同步
Plugin Runtime
```

原因：

> 本阶段目标是验证 Generic Work Model，而不是做媒体管理软件。

---

# 28. 本阶段允许的智能能力

允许：

```text
目录候选发现
名称归一化
已有 Work 匹配
Missing Asset 检测
重复目录检测
基础关系建议
```

这些属于：

```text
Deterministic / Heuristic Intelligence
```

不需要 LLM。

---

# 29. 必须增加 Generic Work Regression Tests

至少新增：

```text
test_anime_work_lifecycle
test_manga_work_lifecycle
test_book_work_lifecycle
```

测试不需要专门创建新表。

重点验证：

```text
WorkType
Progress
Asset
ExternalReference
Memory
Collection
```

全部能够复用。

---

# 30. 必须增加“无本地 Asset”测试

这是非常重要的回归测试。

例如：

```text
create Work(type=anime)
add ExternalReference
update Progress
add Memory
```

整个过程：

```text
assets = 0
```

仍然应该：

```text
PASS
```

最终：

```text
Work
Progress
Reference
Memory
```

全部存在。

---

# 31. 必须增加“本地 + 外链混合”测试

例如：

```text
BLEACH
├── local directory
├── cover
├── Bangumi URL
├── Official URL
├── Progress
└── Memory
```

重新打开数据库之后：

```text
all relations intact
```

必须 PASS。

---

# 32. 必须增加“同一 Work 多来源”测试

例如：

```text
BLEACH
├── local directory A
├── local directory B
├── Bangumi
├── Wikipedia
└── Official
```

必须确认：

```text
1 Work
N Assets
N ExternalReferences
```

而不是：

```text
BLEACH-A
BLEACH-B
```

被错误拆成多个作品。

---

# 33. Missing Asset 语义保持不破坏数据

已经完成的：

```text
directory missing
    ↓
AssetStatus::Missing
```

必须保持：

```text
Work exists
Relation exists
Asset exists
```

不能因为本地目录消失而：

```text
delete Work
delete Relation
```

这条规则适用于：

```text
Game
Anime
Manga
Book
Project
```

所有 Work。

---

# 34. Relocate 应该保持 Generic

当前 Game 已经拥有：

```text
Relocate
```

后续不要实现：

```text
relocate_game
```

而应该最终抽象为：

```text
relocate_asset
```

或者：

```text
relocate_work_directory
```

让所有 WorkType 都可以复用。

---

# 35. Web Demo 继续保持 Adapter 隔离

GitHub Pages：

```text
webAdapter.ts
```

可以继续提供：

```text
demo data
```

但必须保证：

```text
Desktop
   ↓
Tauri
   ↓
LoomaCore
   ↓
SQLite
```

不会加载：

```text
webAdapter
```

未来增加：

```text
Anime
Manga
Book
```

也必须遵守这一边界。

---

# 36. UI 信息架构建议

最终左侧导航可以逐渐从：

```text
数字资产
实体概念
集合收藏
记忆手记
个人游戏库
```

向：

```text
数字资产

作品
  ├── 全部
  ├── 游戏
  ├── 动画
  ├── 漫画
  ├── 书籍
  └── 电影

实体概念

集合收藏

记忆手记
```

演进。

注意：

> “游戏 / 动画 / 漫画”应该是 Work 的 View / Filter，而不是独立数据库模块。

---

# 37. 最终目标模型

最终希望形成：

```text
                         Looma
                           │
                         Work
                           │
       ┌───────────────┬───┴───────────────┐
       ↓               ↓                   ↓
     Game            Anime               Manga
       │               │                   │
       └───────────────┼───────────────────┘
                       ↓
                 Generic Work
                       │
        ┌──────────────┼──────────────┐
        ↓              ↓              ↓
      Asset         Progress       Reference
        │              │              │
        └──────────────┼──────────────┘
                       ↓
                    Memory
                       │
                   Collection
```

---

# 38. 本补丁完成标准

完成后必须满足：

```text
[ ] Game 继续全部 PASS
[ ] Anime 可以真实创建
[ ] Manga 可以真实创建
[ ] Book 可以真实创建
[ ] Movie 可以真实创建
[ ] 不新增专属数据库表
[ ] 不新增 Game/Anime/Manga Service
[ ] Progress 完全复用
[ ] Asset 完全复用
[ ] Collection 完全复用
[ ] Memory 完全复用
[ ] ExternalReference 完全复用
[ ] CLI 使用统一 Work 命令
[ ] MCP 使用统一 Work 工具
[ ] GUI 使用统一 Work 数据链路
[ ] 无本地 Asset 的 Work 可以存在
[ ] 有本地 Asset 的 Work 可以存在
[ ] 有外链但无本地 Asset 的 Work 可以存在
[ ] 本地 + 外链 + Progress + Memory 可以同时存在
[ ] Missing Asset 不会删除 Work
[ ] Relocate 不会改变 Work Identity
```

---

# 39. 明确的架构红线

反重力实施过程中，如果出现以下任何情况，立即停止并重新设计：

```text
❌ AnimeTable
❌ MangaTable
❌ BookTable
❌ GameTable

❌ AnimeService
❌ MangaService
❌ GameService

❌ AnimeProgress
❌ MangaProgress
❌ GameProgress

❌ AnimeAsset
❌ MangaAsset
❌ GameAsset

❌ GUI 直接访问 SQLite
❌ CLI 自己实现一套数据库逻辑
❌ MCP 绕过 LoomaCore
❌ 为每种 WorkType 创建独立 CRUD
```

允许：

```text
✅ WorkType
✅ WorkMetadata
✅ WorkProgress
✅ WorkSummary
✅ Type-aware GUI presentation
✅ Generic Candidate Detection
✅ Generic Asset Linking
```

---

# 40. 本阶段最终定位

Looma 不是：

> 游戏库 + 动漫库 + 漫画库 + 书库的集合。

而是：

> **一个统一的 Personal Work / Asset / Memory Graph。**

Game、Anime、Manga、Book 只是：

```text
WorkType
```

而不是不同的软件。

最终：

```text
一个人拥有：

1000 个文件
200 个游戏
300 部动画
150 部漫画
50 本书
几十个项目
大量照片
大量笔记
大量网页收藏

```

这些东西不应该各自形成孤岛。

它们最终应该进入同一张个人数字资产图谱：

```text
                       Looma
                         │
                      Work Graph
                         │
       ┌─────────────────┼─────────────────┐
       ↓                 ↓                 ↓
     Works             Assets           Memories
       │                 │                 │
       └─────────────────┼─────────────────┘
                         ↓
                    Relationships
                         │
               ┌─────────┼─────────┐
               ↓         ↓         ↓
          Collections References Progress
```

**本补丁的最终目标不是“增加几个媒体类型”，而是证明 Looma 的 Work 架构已经足够通用，可以持续承载你的真实人生数字资产，而不需要每增加一种东西就重做一次数据库。**