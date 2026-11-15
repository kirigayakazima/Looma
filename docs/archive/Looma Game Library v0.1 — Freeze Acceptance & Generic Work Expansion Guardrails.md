# Looma Game Library v0.1
## Freeze Acceptance & Generic Work Expansion Guardrails

> Status: **FINAL ACCEPTANCE**
>
> Target: Antigravity / Looma Coding Agent
>
> Purpose: 对 Game Library v0.1 Post-Patch Final 审计结果进行最终验收，并锁定后续 Generic Work Type Expansion 的架构边界。
>
> Important: 本文不是新的 Game 功能开发任务。

---

# 1. Final Decision

根据：

```text
Looma Game Library Implementation Audit Report
(v0.1 — Post-Patch Final)
```

当前 Game Library v0.1 已完成：

```text
P0 Core Architecture:       10 / 10 PASS
P1 Robustness:               7 / 7 PASS
Regression Tests:            8 / 8 PASS
Build / Verification:        PASS
```

最终状态：

```text
Game Library v0.1 = FROZEN
```

不再继续增加 Game-specific architecture。

---

# 2. What Has Been Proven

当前已经证明：

```text
Game
  ↓
Work(type = game)
  ↓
Asset
  ↓
Directory Asset
  ↓
Relation
  ↓
SQLite
```

是真实的数据模型，而不是 UI Mock。

同时：

```text
GUI
CLI
MCP
```

共享：

```text
LoomaCore
```

以及同一份持久化数据。

因此 Game Library 已经达到：

> **作为 Looma Generic Work Model 第一个真实落地实例的标准。**

---

# 3. P0 Data Integrity — Accepted

以下问题已经正式关闭：

```text
dummyAssetId
fake Asset
orphan Relation
frontend-generated Asset ID
```

当前正确流程：

```text
Directory Path
      ↓
normalize_directory_path()
      ↓
ensure_directory_asset()
      ↓
real AssetKind::Directory
      ↓
link_work_directory()
      ↓
Relation
      ↓
SQLite transaction
```

该链路保持不变。

---

# 4. P1 Missing Asset — Accepted

当前：

```text
Physical Directory Missing
```

不会导致：

```text
Work deleted
Relation deleted
Asset deleted
```

而是：

```text
AssetStatus::Missing
```

同时 GUI：

```text
Missing Warning
      +
Relocate
```

这一行为符合 Local-First Library 的长期语义。

正式冻结。

---

# 5. Game Library Architecture — Freeze

从现在开始禁止为了 Game Library 增加：

```text
GameTable
GameEntity
GameService
GameRepository
GameManager
GameProgressTable
GameInstallPath
GameMetadataTable
```

Game 的唯一 Domain 身份仍然是：

```text
Work(type = game)
```

---

# 6. Important Architectural Interpretation

Game Library 不应该被理解为：

```text
Looma
 ├── Game System
 ├── Anime System
 ├── Manga System
 └── Book System
```

而应该理解为：

```text
Looma
  │
  └── Generic Work System
          │
          ├── game
          ├── anime
          ├── manga
          ├── book
          ├── movie
          ├── tv
          └── project
```

Game 只是第一个已经完整验证的 Work Type。

---

# 7. Generic Work Expansion — Next Phase

下一阶段正式进入：

```text
Generic Work Type Expansion
```

但第一步：

> **只做架构设计，不立即实现全部 Work Type。**

目标是回答：

```text
What makes a Work type-specific?
```

以及：

```text
What should remain generic?
```

---

# 8. Generic Layer

以下能力默认属于所有 Work：

```text
Work
Asset
Relation
Progress
Memory
Collection
ExternalReference
Timeline
Metadata
Status
```

例如：

```text
Game
Anime
Manga
Book
Movie
TV
Project
```

都可以拥有：

```text
Progress
Memory
Collection
ExternalReference
Asset
```

---

# 9. Work Type Specific Layer

只有真正具有类型特异性的内容，才允许进入 Type-specific metadata。

例如：

```text
Anime:
    episode
    season

Manga:
    chapter
    volume

Game:
    platform
    save/location semantics

Book:
    page
    edition

Movie:
    runtime
```

但是不要立刻创建：

```text
AnimeTable
MangaTable
GameTable
BookTable
```

优先研究：

```text
Work metadata
Typed properties
Schema / metadata definitions
```

是否足以表达这些差异。

---

# 10. Example — Anime

未来：

```text
BLEACH 千年血战篇
```

应该首先是：

```text
Work {
    type: anime
}
```

然后：

```text
Progress
    position = 25
    position_type = episode
```

可能存在：

```text
Asset
    poster
    local video directory
```

以及：

```text
ExternalReference
    Bangumi
    Official Website
```

和：

```text
Memory
    personal review
```

以及：

```text
Collection
    "我喜欢的动漫"
```

---

# 11. Example — Manga

例如：

```text
BLEACH
```

应该是：

```text
Work {
    type: manga
}
```

而不是：

```text
MangaEntity
MangaTable
```

可以拥有：

```text
Progress
    position = chapter 686
    position_type = chapter
```

同时：

```text
Asset
    local comic directory
```

以及：

```text
ExternalReference
    official
    Bangumi
```

---

# 12. Example — Game

当前已经验证：

```text
NieR:Automata
```

可以是：

```text
Work(type = game)
```

拥有：

```text
Progress
Asset
ExternalReference
Memory
Collection
Relation
```

因此 Game 不需要特殊的 architecture。

---

# 13. Most Important Rule

未来新增 Work Type 时：

先问：

```text
Can this be represented using existing generic Work capabilities?
```

如果答案是：

```text
YES
```

不要创建新的 Domain system。

只有当答案是：

```text
NO
```

并且确认存在真正稳定的类型特异性需求时，才考虑扩展 Generic Work Model。

---

# 14. Avoid Premature Specialization

禁止出现这种趋势：

```text
Game
 ├── GameProgress
 ├── GameAsset
 └── GameReference

Anime
 ├── AnimeProgress
 ├── AnimeAsset
 └── AnimeReference

Manga
 ├── MangaProgress
 ├── MangaAsset
 └── MangaReference
```

正确方向：

```text
                 Work
                   │
       ┌───────────┼───────────┐
       │           │           │
     Game        Anime       Manga
       │           │           │
       └───────────┼───────────┘
                   │
          Generic Capabilities
                   │
       ┌───────────┼───────────┐
       │           │           │
     Asset      Progress    Reference
       │           │           │
       └───────────┼───────────┘
                   │
                Memory
                   │
              Collection
```

---

# 15. Recommended Next Deliverable

下一份文档不要直接叫：

```text
Anime Implementation
```

或者：

```text
Manga Implementation
```

而应该先建立：

```text
docs/
└── Looma Generic Work Type Model Design v0.1.md
```

这份文档至少需要回答：

```text
1. Work Type 的定义
2. Generic Work capabilities
3. Type-specific metadata
4. Progress model
5. Asset model
6. ExternalReference model
7. Relation model
8. Memory model
9. Collection model
10. Timeline model
11. Schema evolution
12. Migration strategy
13. GUI rendering strategy
14. CLI representation
15. MCP representation
```

---

# 16. Do Not Implement Yet

在 Generic Work Type Model Design 完成之前：

不要直接实现：

```text
Anime UI
Manga UI
Book UI
Movie UI
```

也不要增加：

```text
anime.rs
manga.rs
book.rs
```

等价的平行 Domain。

先把 Generic Model 设计清楚。

---

# 17. Regression Requirement

Generic Work Expansion 开始以后：

> **Game Library v0.1 必须成为 Regression Baseline。**

任何新的 Work Type 修改都必须确保：

```text
Game tests remain PASS
```

尤其是：

```text
Directory Asset
Multi-location
Progress
Collection
ExternalReference
Missing Asset
GUI / CLI / MCP consistency
Restart persistence
```

不得因为 Anime/Manga 扩展破坏 Game。

---

# 18. Freeze Boundary

Game Library v0.1 当前边界：

```text
                    Looma
                      │
                      ▼
              Generic Work
                      │
              ┌───────┴───────┐
              │               │
          Work(type=game)   Other types
              │
       ┌──────┼─────────────┐
       │      │             │
     Asset Progress      Reference
       │      │             │
       └──────┼─────────────┘
              │
        Memory / Collection
```

Game 侧：

```text
FROZEN
```

Generic Work：

```text
NEXT DESIGN TARGET
```

---

# 19. Final Acceptance Statement

如果当前 Post-Patch Final 审计报告中的代码证据和测试结果均与实际仓库一致，则：

```text
Looma Game Library v0.1
STATUS = ACCEPTED
STATUS = FROZEN
```

不再对 Game Library 做结构性修改。

后续工作正式从：

```text
"Build the Game Library"
```

转向：

```text
"Generalize the validated Work Model"
```

最终目标：

```text
Game
Anime
Manga
Book
Movie
TV
Project
```

不是七套系统。

而是：

```text
              ┌──────────────┐
              │     Work     │
              └──────┬───────┘
                     │
        ┌────────────┼────────────┐
        │            │            │
      Game         Anime        Manga
        │            │            │
        └────────────┼────────────┘
                     │
              Generic Looma
              Capabilities
```

**Game Library v0.1 到此结束。**
**下一阶段只研究 Generic Work Type Model，不再继续堆 Game 功能。**