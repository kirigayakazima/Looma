# Looma Game Library v0.1
## P0/P1 Architecture & Data Integrity Patch

> Status: **Implementation Patch**
>
> Target: Antigravity / Looma Coding Agent
>
> Scope: 修复 Game Library v0.1 审计发现的问题，并在完成后冻结 Game Library v0.1。
>
> Audit Basis: `Looma Game Library Implementation Audit v0.1`

---

# 1. Patch Goal

本补丁不是继续扩展 Game Library 功能，而是：

1. 修复 `dummyAssetId` 导致的虚假 Asset Relation。
2. 确保所有 Game → Directory Asset Relation 都指向真实存在的 `assets` 记录。
3. 增加 Directory Asset 的幂等创建 / 查找能力。
4. 补齐本地目录丢失时的 UI 状态。
5. 增加覆盖这些场景的回归测试。
6. 完成后冻结 Game Library v0.1。
7. 为下一阶段的 `Work Type` 扩展保留干净架构，但**本补丁不实现 Anime/Manga 等新类型**。

核心原则：

```text
GUI
  ↓
Tauri Command
  ↓
LoomaCore
  ↓
Domain / Database
```

任何 GUI 行为都不得绕过 `LoomaCore` 直接操作 SQLite。

---

# 2. Current Architecture — Keep This

当前 Game Library 的架构是正确的，不要重新设计：

```text
Looma
 │
 ├── Work
 │    └── type = game
 │
 ├── Asset
 │    ├── Directory
 │    ├── File
 │    └── ...
 │
 ├── WorkProgress
 │
 ├── Memory
 │
 ├── Collection
 │
 ├── ExternalReference
 │
 └── Relation
```

Game 本质仍然只是：

```text
Work(type = game)
```

禁止新增：

```text
GameTable
GameEntity
GameService
GameManager
GameRepository
Game-specific persistence layer
```

不要为了修复本次问题引入 Game 专用数据模型。

---

# 3. P0 — Remove `dummyAssetId`

## 3.1 Problem

当前 Game Candidate → Game Link 的部分前端流程存在：

```text
candidate
  ↓
dummyAssetId
  ↓
relation(work → dummyAssetId)
```

例如：

```text
asset_dir_${candidate.id}
```

这个 ID 可能根本不存在于 `assets` 表。

最终形成：

```text
entities
  └── Game

relations
  └── Game ──attaches──> asset_dir_xxx

assets
  └── 不存在 asset_dir_xxx
```

这属于真正的数据完整性问题。

后续：

```text
get_asset(asset_dir_xxx)
```

得到：

```text
None
```

于是：

- Game Detail 无法正常显示目录；
- Relation 指向不存在实体；
- 数据库逻辑上存在“幽灵关系”；
- GUI 显示和数据库状态可能不一致。

---

# 4. Required Fix — `ensure_directory_asset`

新增一个通用的 Directory Asset 确保入口。

推荐位置：

```text
LoomaCore
```

或者由：

```text
LoomaCore → Asset persistence layer
```

提供。

建议接口：

```rust
ensure_directory_asset(path: &str) -> Result<AssetId>
```

具体命名可以根据当前 Core API 风格调整，但语义必须保持一致。

---

# 5. Required Behavior

输入：

```text
E:/Games/NieR Automata
```

执行：

```text
ensure_directory_asset(path)
```

必须：

### Case A — Asset 已存在

返回已有 Asset：

```text
AssetKind::Directory
path = normalized path
```

不能创建第二份。

---

### Case B — Asset 不存在，但目录存在

创建：

```text
Asset {
    id: generated_id,
    kind: Directory,
    path: normalized_path,
    mime_type: "inode/directory"
}
```

然后返回 Asset ID。

---

### Case C — Path 不存在

这里不要制造一个假的 Directory Asset。

建议返回明确错误：

```text
DirectoryNotFound
```

或者项目现有等价错误。

不能：

```text
create fake Asset
```

也不能：

```text
create relation → nonexistent asset
```

---

# 6. Path Normalization

`ensure_directory_asset()` 必须进行稳定的路径规范化。

Windows 环境至少考虑：

```text
E:/Games/Test
E:\Games\Test
e:/Games/Test
```

不能因为表现形式不同而重复创建：

```text
Asset A → E:/Games/Test
Asset B → E:\Games\Test
```

推荐统一：

```text
normalize_directory_path()
```

然后：

```text
lookup normalized path
        ↓
existing Asset?
   ├── yes → return existing ID
   └── no  → create Asset
```

不要通过随机 Asset ID 判断目录是否重复。

**路径才是 Directory Asset 的自然去重依据。**

如果当前数据库已经存在适合的 unique constraint / index，则复用。

如果没有，优先考虑添加数据库层面的唯一约束，而不是只依赖 Rust 代码判断。

---

# 7. Relation Creation Must Be Transactionally Safe

Game Link 的正确流程应该变成：

```text
Candidate
   ↓
validate directory
   ↓
normalize path
   ↓
ensure_directory_asset()
   ↓
real Asset ID
   ↓
create Relation
   ↓
commit
```

而不是：

```text
Candidate
   ↓
generate dummy ID
   ↓
create Relation
```

理想情况下：

```text
BEGIN TRANSACTION

ensure directory asset
        ↓
create relation

COMMIT
```

如果任一步骤失败：

```text
ROLLBACK
```

不能留下：

```text
orphan relation
```

也不能留下半完成状态。

---

# 8. Candidate Confirmation Must Use Real Asset

重点检查：

```text
GamesView.tsx
```

以及 Candidate confirmation / quick-link 相关逻辑。

如果当前存在类似：

```typescript
const dummyAssetId = `asset_dir_${candidate.id}`;
```

或者任何等价逻辑：

```typescript
const assetId = ...
```

然后直接：

```typescript
relateWork(workId, assetId)
```

必须移除。

正确逻辑：

```text
candidate.path
    ↓
Tauri command
    ↓
LoomaCore.ensure_directory_asset(path)
    ↓
real assetId
    ↓
create relation
```

前端不负责决定 Asset ID。

---

# 9. Recommended Tauri API

如果当前架构需要通过 Tauri 暴露能力，可以增加类似：

```rust
#[tauri::command]
async fn ensure_directory_asset(
    path: String,
    state: State<'_, AppState>
) -> Result<AssetDto, String>
```

或者直接返回：

```rust
AssetId
```

以当前项目 DTO/API 风格为准。

关键要求不是具体函数名，而是：

> **Frontend never manufactures Asset IDs.**

Asset identity 必须由 Core / Persistence 层决定。

---

# 10. Linking a Game Must Become Generic

建议最终形成这样的通用能力：

```text
link_work_to_asset(work_id, asset_id)
```

而不是：

```text
link_game_to_directory(...)
```

Game Library 只是：

```text
Work(type = game)
        +
Asset(kind = directory)
        +
Relation(type = attaches)
```

这也是未来 Anime / Manga / Book / Movie 等 Work Type 可以复用的关键。

---

# 11. P1 — Missing Directory UI

当前后端已经能够识别本地 Asset 缺失状态。

但是 Game Detail UI 对：

```text
Directory no longer exists
```

缺少足够明显的视觉反馈。

需要补齐。

---

# 12. Missing Asset Display

当：

```text
Asset.kind = Directory
```

但是：

```text
path does not exist
```

Game Detail 中必须显示明确状态，例如：

```text
⚠ Directory Missing

E:/Games/NieR Automata

This location is no longer available.
```

具体 UI 风格可以遵循现有设计系统。

重点不是文字必须完全一样，而是：

> 用户必须一眼知道这个 Game 的本地目录已经丢失。

---

# 13. Missing ≠ Deleted Work

非常重要：

目录不存在：

```text
E:/Games/NieR Automata
```

不能导致：

```text
DELETE Work
```

也不能自动删除：

```text
Relation
```

正确状态：

```text
Work
 ├── exists
 ├── progress exists
 ├── memories exist
 ├── external references exist
 └── Asset
      └── missing
```

Game 本身仍然存在。

这是 Local-First Digital Library 的重要语义。

---

# 14. Relocate / Update Path

如果当前 Game Detail 已经有：

```text
Add Location
```

可以继续复用。

建议支持：

```text
Missing Directory
       ↓
Relocate / Update Location
       ↓
Select new directory
       ↓
ensure_directory_asset(new_path)
       ↓
link new Asset
```

不要删除原来的 Work。

也不要破坏历史 Relation。

如果当前版本实现成本过高：

```text
P1 optional
```

至少必须完成：

```text
Missing badge
```

---

# 15. Multi-Location Must Continue Working

以下状态必须合法：

```text
Work: NieR Automata

Assets:

1. E:/Games/NieR Automata
2. D:/Backup/NieR Automata
3. F:/SSD/Games/NieR Automata
```

关系：

```text
Game
 ├── attaches → Asset A
 ├── attaches → Asset B
 └── attaches → Asset C
```

不能改成：

```text
Game → one directory only
```

也不要给 Game 增加：

```text
game.install_path
```

这样的 Game 专用字段。

---

# 16. Required Tests

必须增加回归测试。

## Test 1 — Link Unindexed Directory

前置：

```text
Directory exists
Asset row does not exist
Work exists
```

执行：

```text
link work → directory
```

期望：

```text
assets row created
AssetKind::Directory
path correct

relation created
relation.target_id == real asset.id

get_asset(asset.id) != None
```

---

# 17. Test 2 — Link Same Directory Twice

第一次：

```text
ensure_directory_asset("E:/Games/Test")
```

得到：

```text
Asset A
```

第二次：

```text
ensure_directory_asset("E:/Games/Test")
```

期望：

```text
Asset A
```

而不是：

```text
Asset B
```

最终：

```text
assets count = 1
```

---

# 18. Test 3 — Path Format Deduplication

测试：

```text
E:/Games/Test
E:\Games\Test
```

经过 normalization 后：

```text
same directory identity
```

不得生成两个 Directory Assets。

Windows 大小写行为按照当前项目已有 path semantics 决定，但必须保持稳定、一致。

---

# 19. Test 4 — Multiple Locations

创建：

```text
E:/Games/GameA
D:/Games/GameA
F:/Backup/GameA
```

Link 到同一个 Work。

期望：

```text
one Work
three Directory Assets
three valid relations
```

---

# 20. Test 5 — Candidate Confirmation

模拟：

```text
Scanner detects:

E:/Games/GameA
```

确认 Candidate。

期望：

```text
Candidate
  ↓
ensure_directory_asset
  ↓
real Asset
  ↓
Relation
```

禁止：

```text
dummyAssetId
```

测试中应明确检查：

```text
relation.target_id exists in assets
```

---

# 21. Test 6 — Restart Persistence

执行：

```text
create/link Game
close Core / reconnect database
reload
```

期望：

```text
Game exists
Asset exists
Relation exists
Directory path unchanged
```

---

# 22. Test 7 — Missing Directory

创建：

```text
Game
  ↓
Directory Asset
```

然后让目录不可访问 / 删除测试目录。

重新加载。

期望：

```text
Work still exists
Asset still exists
Relation still exists
Asset status = missing
UI shows missing warning
```

不能：

```text
delete Work
```

---

# 23. Test 8 — No Orphan Relation

建立一条 Game → Directory Relation 后：

```text
assert target Asset exists
```

同时测试所有 Game linking code path：

```text
Candidate confirmation
Quick add
Manual add location
CLI
MCP
```

都不能产生：

```text
relation → nonexistent asset
```

---

# 24. Scanner Responsibility

Scanner 当前设计：

```text
read-only discovery
```

保持。

Scanner 可以：

```text
detect_game_candidates()
```

但是不要让 Scanner 自己偷偷创建：

```text
Work
Asset
Relation
```

推荐职责：

```text
Scanner
    ↓
Candidate
    ↓
User confirmation
    ↓
LoomaCore mutation
```

保持这个边界。

---

# 25. Do Not Over-Fix Scanner

当前通用 Scanner：

```rust
if !entry.file_type().is_file() {
    continue;
}
```

因此它主要把目录作为 traversal container，而不是自动注册 Directory Asset。

本补丁**不要求**为了 Game Library 修改整个 Scanner。

优先采用：

```text
ensure_directory_asset(path)
```

解决 Game linking 的真实需求。

如果未来发现 Directory Asset indexing 本身应该成为通用能力，再单独设计。

不要为了本次 bug 做大规模 Scanner 重构。

---

# 26. CLI / MCP Consistency

CLI：

```text
looma work relate <WORK_ID> <ASSET_ID>
```

保持。

如果 CLI 提供：

```text
link directory
```

也必须最终走：

```text
LoomaCore
```

MCP 同理。

不能出现：

```text
GUI → Core
CLI → Database
MCP → Database
```

必须保持：

```text
GUI ─┐
CLI ─┼→ LoomaCore → Database
MCP ─┘
```

---

# 27. Data Integrity Invariant

完成本补丁后，加入并明确这个 invariant：

```text
For every relation:

relation.target_id
    MUST reference an existing asset
```

对于：

```text
Work --attaches--> Asset
```

必须保证：

```text
Work exists
AND
Asset exists
AND
Relation exists
```

不能接受：

```text
Work exists
AND
Relation exists
BUT
Asset does not exist
```

---

# 28. Forbidden Implementation

以下实现禁止：

### ❌ Fake Asset ID

```typescript
`asset_dir_${candidate.id}`
```

---

### ❌ Frontend-created Asset IDs

```typescript
const assetId = randomUUID();
```

然后直接创建 Relation。

---

### ❌ Game-specific Asset Table

```text
game_assets
```

---

### ❌ Game-specific path field

```text
games.install_path
games.install_paths
```

---

### ❌ Game-specific manager

```text
GameManager
GameService
GameRepository
```

---

### ❌ Automatically delete Game when directory disappears

```text
missing directory → delete game
```

---

### ❌ Direct SQLite access from React

```text
React → SQLite
```

---

# 29. Acceptance Criteria

本补丁完成后必须满足：

```text
[PASS] No dummyAssetId remains in production linking path

[PASS] Linking an unindexed existing directory creates
       a real AssetKind::Directory

[PASS] Every Work → Asset relation points to a real Asset

[PASS] Same directory is idempotent

[PASS] Multiple locations work

[PASS] Candidate confirmation uses real Asset

[PASS] Restart preserves Work / Asset / Relation

[PASS] Missing directory does not delete Work

[PASS] Missing directory has visible UI warning

[PASS] GUI / CLI / MCP continue through LoomaCore

[PASS] No Game-specific persistence architecture added

[PASS] Regression tests pass
```

---

# 30. Verification Commands

完成后执行项目当前标准测试流程。

至少覆盖：

```text
cargo test
```

以及项目已有：

```text
database tests
core tests
scanner tests
CLI tests
MCP tests
desktop build/type checks
```

如果项目存在已有专项测试命令，优先使用项目既有命令。

最终报告：

```text
Test Summary
------------
Core:      PASS
Database:  PASS
Scanner:   PASS
CLI:       PASS
MCP:       PASS
Desktop:   PASS

Game Library Regression
-----------------------
Unindexed directory: PASS
Duplicate directory: PASS
Multi-location: PASS
Candidate linking: PASS
Restart persistence: PASS
Missing directory: PASS
No orphan relation: PASS
```

---

# 31. Game Library v0.1 Freeze

完成以上修复后：

> **Game Library v0.1 should be considered frozen.**

后续除：

```text
bug fix
data integrity
performance
UX polish
```

之外，不再继续向 Game 增加独立业务系统。

不要继续添加：

```text
GameManager
Game-specific database tables
Game-specific metadata architecture
Game-specific progress architecture
Game-specific collection architecture
Game-specific memory architecture
```

因为这些能力已经属于 Looma 的 Generic Work Model。

---

# 32. Next Architecture Direction

Game v0.1 冻结之后，下一阶段应该研究：

# Generic Work Type Expansion

目标不是：

```text
Game
Anime
Manga
Book
Movie
TV
Project
```

分别建立系统。

而是：

```text
Work
 │
 ├── type = game
 ├── type = anime
 ├── type = manga
 ├── type = book
 ├── type = movie
 ├── type = tv
 └── type = project
```

所有 Work 共享：

```text
Asset
Progress
Memory
Collection
ExternalReference
Relation
Timeline
```

例如：

```text
BLEACH 千年血战篇
    Work(type=anime)
        │
        ├── Progress
        ├── ExternalReference
        ├── Asset
        ├── Memory
        └── Collection
```

以及：

```text
NieR:Automata
    Work(type=game)
        │
        ├── Progress
        ├── ExternalReference
        ├── Asset
        ├── Memory
        └── Collection
```

---

# 33. Important — Design Only

**这一阶段暂时不要实现 Anime / Manga / Book。**

只需要保证当前 Game 修复不会破坏未来：

```text
Work(type = *)
```

的通用性。

下一阶段再单独产生：

```text
Looma Generic Work Content Model Design
```

进行架构设计。

不要在本次 Patch 中顺手实现。

---

# 34. Final Architecture Target

最终希望保持：

```text
                         ┌───────────────┐
                         │   Looma GUI   │
                         └───────┬───────┘
                                 │
                         ┌───────▼───────┐
                         │   LoomaCore   │
                         │    Facade     │
                         └───────┬───────┘
                                 │
             ┌───────────────────┼───────────────────┐
             │                   │                   │
        ┌────▼────┐         ┌────▼────┐        ┌────▼────┐
        │   Work  │         │  Asset  │        │ Relation│
        └────┬────┘         └────┬────┘        └────┬────┘
             │                   │                   │
             └───────────────────┼───────────────────┘
                                 │
                         ┌───────▼───────┐
                         │    SQLite     │
                         └───────────────┘
```

其中：

```text
Work
```

负责“我有什么东西”。

```text
Asset
```

负责“它对应哪些本地数字资产”。

```text
Relation
```

负责“它们之间是什么关系”。

```text
Progress
```

负责“我看到 / 玩到 / 读到哪里”。

```text
Memory
```

负责“我对它留下了什么”。

```text
ExternalReference
```

负责“它在外部世界对应什么”。

这才是 Looma 后续扩展 Anime / Manga / Book / Movie 等内容的基础。

---

# 35. Final Instruction to Agent

请按以下顺序实施：

```text
1. Locate all dummyAssetId / fake Asset ID paths
2. Implement ensure_directory_asset()
3. Normalize + deduplicate directory paths
4. Replace candidate confirmation / quick-link fake IDs
5. Ensure Relation creation only references real Assets
6. Add missing-directory UI state
7. Add regression tests
8. Run full test suite
9. Verify GUI / CLI / MCP boundaries
10. Remove dead dummyAssetId code
11. Produce implementation report
12. STOP
```

完成后不要继续扩展 Game 功能。

**Game Library v0.1 到此冻结。**

下一阶段只进入：

```text
Generic Work Type Expansion
```

的设计阶段。