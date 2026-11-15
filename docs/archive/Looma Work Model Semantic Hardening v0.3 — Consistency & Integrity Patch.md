# Looma Work Model Semantic Hardening v0.3
## Consistency, Atomicity & Compatibility Hardening Patch

> **Patch Version**: v0.3  
> **Previous Baseline**: `Looma Work Model Semantic Hardening v0.2`  
> **Target Scope**: `crates/core`, `crates/database`, `crates/scanner`, `crates/mcp`, `crates/cli`, `apps/desktop`  
> **Primary Goal**: 在 v0.2 五层 Work Model 已经成立的基础上，进一步消除“逻辑正确但事务边界不完整 / 数据可能短暂或永久不一致 / 检索语义过宽 / 兼容性声明过强”等隐患。  
> **Architecture Principle**: 继续坚持 Generic Work Model，不引入任何媒体类型专属表、Service、Progress、Asset 或 Repository。  
> **Schema Principle**: 本补丁默认 **零新增 SQLite 物理表、零新增媒体专属 Migration**。

---

# 0. Patch Objective

v0.2 已经完成：

- Generic Work Entity；
- Identity / Metadata / Lifecycle / Progress / Connections 五层模型；
- Aliases；
- 多 ExternalReference；
- Primary External Reference；
- Status / Progress 正交解耦；
- Universe Relation；
- Missing Asset / Relocate；
- 冷重启持久化；
- GUI / CLI / MCP 统一 LoomaCore。

v0.3 不再扩展 Work 的业务范围，而是进入：

> **Consistency & Integrity Hardening**

本阶段只解决以下六类问题：

1. **Relocate 必须原子化**
2. **Primary External Reference 必须具有严格唯一性**
3. **ExternalReference 主入口切换必须事务安全**
4. **Aliases 检索必须区分“字段命中”与“JSON 偶然命中”**
5. **Status / Metadata / Progress 的序列化兼容性必须被真实测试**
6. **CLI / MCP / GUI 三端必须覆盖同一组异常路径**

---

# 1. Absolute Architecture Guardrails

以下红线继续保持：

## 1.1 禁止专属数据库表

严禁：

```text
AnimeTable
MangaTable
BookTable
MovieTable
GameTable
TvSeriesTable
MusicTable
```

所有 Work 继续进入：

```text
entities
```

---

## 1.2 禁止专属 Service

严禁：

```text
AnimeService
MangaService
BookService
MovieService
GameService
```

统一使用：

```text
LoomaCore
EntityService
RelationService
ExternalReferenceService
```

---

## 1.3 禁止专属 Progress

严禁：

```text
AnimeProgress
MangaProgress
BookProgress
GameProgress
MovieProgress
```

继续统一：

```text
WorkProgress
```

---

## 1.4 禁止专属 Asset

严禁：

```text
AnimeAsset
MangaAsset
BookAsset
GameAsset
```

继续统一：

```text
AssetKind::File
AssetKind::Directory
```

---

# 2. P0 — Relocate Atomicity Hardening

## 2.1 当前风险

v0.2 当前实现类似：

```rust
let (new_asset, new_rel) =
    self.link_work_directory(actor, work_id, new_dir_path)?;

let _ =
    self.delete_relation_between(actor, work_id, old_asset_id);

let _ =
    self.delete_relation_between(actor, old_asset_id, work_id);
```

这里存在一个重要问题：

```text
创建新关系
    ↓
删除旧关系
    ↓
删除失败被忽略
```

因此理论上可能产生：

```text
Work
 ├── old directory
 └── new directory
```

也就是说：

> Relocate 可能退化为 Add，而不是严格意义上的 Move。

更严重的是：

```rust
let _ = ...
```

会吞掉删除失败。

---

# 3. P0 — Relocate 必须成为单一原子事务

重新定义：

```text
relocate_work_directory()
```

语义：

> 将 Work 从 old_asset_id 原子地迁移到 new_dir_path。

必须满足：

```text
BEGIN TRANSACTION

1. Validate Work
2. Validate old relation
3. Validate new directory
4. ensure/create new Asset
5. create new Work -> Asset relation
6. remove old Work -> Asset relation
7. validate final relation state
8. record audit

COMMIT
```

任何一步失败：

```text
ROLLBACK
```

不得留下：

```text
new relation only
old relation only
both relations
partial Asset
```

---

# 4. P0 — Relocate Failure Semantics

以下情况必须明确返回错误：

### Case A — Work 不存在

```text
NotFound
```

### Case B — old_asset_id 不存在

```text
NotFound
```

### Case C — old_asset_id 并未关联该 Work

```text
Validation / NotFound
```

### Case D — new path 不存在

```text
Validation
```

### Case E — new path 与旧 path 实际为同一路径

不得创建重复关系。

结果应为：

```text
idempotent success
```

### Case F — 新 Asset 已经存在且已经关联 Work

不得产生重复 Relation。

---

# 5. P0 — Relocate Identity Guarantee

Relocate 前后必须保证：

```text
work_id == work_id
```

且：

```text
Work entity 不发生重建
Work metadata 不发生变化
Work status 不发生变化
Work progress 不发生变化
ExternalReference 不发生变化
Memory 不发生变化
Collection membership 不发生变化
```

唯一允许变化：

```text
old local directory relation
        ↓
new local directory relation
```

---

# 6. P0 — Primary External Reference Atomicity

## 6.1 目标

一个 Work 可以拥有：

```text
ExternalReference A
ExternalReference B
ExternalReference C
```

但：

```text
最多只能有一个 is_primary = true
```

允许：

```text
0 primary
1 primary
```

禁止：

```text
2+ primary
```

---

# 7. P0 — set_primary_external_reference 必须事务化

目标行为：

```text
BEGIN

1. Validate entity exists
2. Validate reference exists
3. Validate reference belongs to entity
4. Set all entity references:
       is_primary = false
5. Set target:
       is_primary = true
6. Verify exactly one primary
7. Record audit

COMMIT
```

任何步骤失败：

```text
ROLLBACK
```

不得出现：

```text
all false
```

或：

```text
A = true
B = true
```

---

# 8. P0 — Primary Reference Ownership Check

必须禁止：

```text
Entity A
 └── Ref A1

Entity B
 └── Ref B1
```

执行：

```text
set_primary_external_reference(A, B1)
```

必须失败。

不得因为：

```text
reference_id
```

是合法 ID 就允许跨 Entity 修改。

必须验证：

```text
reference.entity_id == target_entity_id
```

---

# 9. P1 — ExternalReference Creation Semantics

当：

```text
create_external_reference(..., is_primary = true)
```

时：

如果该 Entity 已存在 Primary：

```text
Old Primary → false
New Reference → true
```

整个操作必须在同一事务中完成。

如果：

```text
is_primary = false
```

则：

```text
existing primary 保持不变
```

---

# 10. P1 — Primary Reference Delete Semantics

删除当前 Primary 时必须明确行为。

推荐：

```text
delete primary
    ↓
remaining references
    ↓
all is_primary = false
```

不要隐式随机选择新的 Primary。

也就是说：

> Primary 是用户明确选择的入口，而不是系统自行猜测。

如果未来需要“自动选主入口”，另行设计，不在本 Patch 中实现。

---

# 11. P1 — Alias Search Semantic Hardening

v0.2 当前 SQL：

```sql
AND (
    title LIKE ?
    OR description LIKE ?
    OR properties_json LIKE ?
)
```

虽然能够命中 aliases，但：

```text
properties_json LIKE '%xxx%'
```

语义过宽。

它可能命中：

```text
description
cover_asset_id
external reference
metadata
future unknown fields
```

因此：

> `properties_json LIKE` 只能作为兼容 fallback，不能被视为严格 Alias Query。

---

# 12. P1 — Alias Search 分层策略

推荐统一定义：

```text
Work Search
```

分成：

```text
Identity Search
    title
    original_title
    aliases

General Search
    title
    original_title
    aliases
    description
```

其中：

```text
aliases
```

必须通过反序列化后的：

```rust
WorkMetadata.aliases
```

进行语义匹配。

---

# 13. P1 — SQLite LIKE 的职责限制

如果当前 SQLite Schema 暂时不允许 JSON 专用索引：

可以保留：

```sql
properties_json LIKE ?
```

但必须明确：

> 这是 broad fallback，不是 Alias-specific query。

最终业务语义必须由：

```rust
Entity::matches_query()
```

统一校验。

即：

```text
SQLite candidate filtering
        ↓
Rust semantic filtering
        ↓
final Work results
```

禁止仅依赖：

```text
properties_json LIKE
```

直接返回最终结果。

---

# 14. P1 — Alias Matching Normalization

Aliases 查询必须统一处理：

```text
大小写
前后空格
Unicode 基础差异
```

例如：

```text
Attack on Titan
attack on titan
 ATTACK ON TITAN
```

应当具有一致的基本匹配语义。

不要在本阶段引入复杂的全文搜索引擎。

---

# 15. P1 — Alias Update Semantics

新增：

```text
update_work_aliases()
```

必须保证：

```text
Work 不存在 → NotFound
```

成功后：

```text
title 不变
original_title 不变
status 不变
progress 不变
assets 不变
references 不变
memories 不变
relations 不变
```

只有：

```text
aliases
```

发生变化。

---

# 16. P1 — Alias Deduplication

Aliases 不应该允许：

```text
["AOT", "AOT", "aot"]
```

推荐标准化为：

```text
["AOT"]
```

至少保证完全相同字符串不会重复。

是否进行大小写归一化可以保持当前项目语义，但必须保证：

```text
same normalized alias
```

不会无限重复。

---

# 17. P1 — Status Compatibility Hardening

v0.2 新增：

```rust
RecordStatus::Archived
```

必须验证旧数据：

```text
properties_json
```

不存在：

```text
status
```

时仍然正常读取。

同时必须验证：

```text
Archived
```

重新序列化后仍保持：

```text
archived
```

---

# 18. Compatibility Clarification

不要再简单宣称：

> “100% 向前与向后兼容”

除非有真实测试证明。

本 Patch 将兼容性拆成：

### Data Backward Compatibility

旧数据库：

```text
没有 aliases
没有 type_metadata
没有 archived
```

新版本可以读取。

### New Data / Old Binary Compatibility

新版本写入：

```text
aliases
archived
new metadata
```

旧版本二进制是否可以读取：

> **必须通过专门测试验证，不能仅凭 serde 行为推断。**

如果没有测试：

应声明：

```text
Not Guaranteed
```

而不是：

```text
100% Compatible
```

---

# 19. P1 — Unknown Enum Compatibility

对于：

```rust
RecordStatus
TypeSpecificMetadata
```

评估是否需要：

```text
#[serde(other)]
```

或者显式 Unknown fallback。

目标：

> 新版本增加枚举值时，旧数据读取不应因为未知值导致整个 Work 无法加载。

注意：

如果 `RecordStatus` 使用 `#[serde(other)]`，必须确认业务语义不会把未知状态错误地映射成合法状态。

如果无法安全 fallback：

则保持严格反序列化，并在兼容性文档中明确声明。

---

# 20. P1 — Universe Relation Integrity

继续使用：

```text
adaptation_of
sequel_of
prequel_of
part_of
spin_off_of
remake_of
based_on
```

本 Patch 不增加新的关系类型。

但必须增加：

```text
Relation target existence
Relation source existence
```

验证。

禁止出现：

```text
Work A
   |
   +── adaptation_of → non-existent entity
```

---

# 21. P1 — Relation Direction Consistency

统一明确：

```text
Anime
  --adaptation_of-->
Novel
```

而不是：

```text
Novel
  --adaptation_of-->
Anime
```

所有新增 Universe Relation API 必须遵循：

```text
source = current work
target = referenced work
```

示例：

```text
create_relation(
    anime_id,
    adaptation_of,
    novel_id
)
```

---

# 22. P1 — Duplicate Relation Protection

相同：

```text
source_id
relation_type
target_id
```

不得重复创建。

例如：

```text
Anime A
  adaptation_of
Novel B
```

只能存在一条。

重复调用应：

```text
idempotent success
```

或者：

```text
Conflict
```

二者择一，但必须统一。

推荐：

```text
idempotent success
```

---

# 23. P1 — Self Relation Protection

禁止：

```text
Work A
  adaptation_of
Work A
```

对于 Universe Relation：

```text
source_id == target_id
```

必须返回：

```text
Validation
```

---

# 24. P1 — Collection / Memory / Reference Isolation

对以下操作增加回归断言：

```text
relocate
set_primary_reference
update_aliases
update_status
update_progress
```

均不得意外改变：

```text
Collection membership
Memory relations
External References
Universe Relations
Local Assets
```

即：

> Work 的单层更新不得产生跨层副作用。

---

# 25. P1 — GUI State Consistency

GUI 必须确保：

### Relocate

执行成功后：

```text
old path → removed
new path → visible
```

执行失败后：

```text
UI state unchanged
```

不能出现：

```text
后端失败
但 UI 乐观更新成功
```

---

### Primary Reference

点击：

```text
Set as Primary
```

失败时：

```text
原 Primary 保持高亮
```

成功后：

```text
旧 Primary → 普通引用
新 Primary → 高亮
```

---

### Alias

添加 Alias 失败时：

```text
UI 不得假装已添加
```

---

# 26. P1 — CLI Error Semantics

以下 CLI 操作必须具有非 0 exit code：

```text
looma work relocate-dir
looma ref primary
looma work alias
```

当：

```text
Work 不存在
Asset 不存在
Reference 不属于 Entity
Path 不存在
Relation 不存在
```

时不得：

```text
exit 0
```

同时不得吞掉 Core 错误。

---

# 27. P1 — MCP Error Semantics

MCP 必须保持：

```text
Core Error
    ↓
MCP structured error
```

禁止：

```text
Core failed
    ↓
MCP success response
```

尤其检查：

```text
relocate_work_directory
set_primary_external_reference
update_work_aliases
```

---

# 28. Required Regression Tests

本 Patch 至少新增以下测试。

## Test I — Relocate Atomicity

```text
test_v03_relocate_atomicity
```

验证：

```text
old relation exists
new path invalid
→ operation fails
→ old relation still exists
→ new relation does not exist
```

---

## Test J — Relocate Failure Does Not Partially Commit

```text
test_v03_relocate_failure_rollback
```

模拟：

```text
new asset creation succeeds
relation update fails
```

最终：

```text
old state restored
```

---

## Test K — Primary Uniqueness

```text
test_v03_primary_reference_uniqueness
```

验证：

```text
A = primary
B = false
C = false

set B primary

A = false
B = true
C = false
```

最终：

```text
count(primary) == 1
```

---

## Test L — Cross Entity Primary Rejection

```text
test_v03_primary_reference_ownership
```

验证：

```text
Entity A
Reference A

Entity B
Reference B

set_primary(A, B)
```

必须失败。

---

## Test M — Primary Creation Replacement

```text
test_v03_primary_reference_creation_replacement
```

验证：

```text
create A primary
create B primary
```

最终：

```text
A = false
B = true
```

---

## Test N — Alias Semantic Search

```text
test_v03_alias_semantic_search
```

验证：

```text
title
original_title
alias
description
```

的匹配行为。

同时验证：

```text
properties_json
```

中非 Alias 字段的偶然字符串不会被错误解释为 Alias 命中。

---

## Test O — Alias Deduplication

```text
test_v03_alias_deduplication
```

输入：

```text
AOT
AOT
AOT
```

最终不得出现重复 Alias。

---

## Test P — Universe Relation Integrity

```text
test_v03_universe_relation_integrity
```

验证：

```text
missing target → reject
self relation → reject
duplicate relation → idempotent
```

---

## Test Q — Cross Layer Isolation

```text
test_v03_cross_layer_update_isolation
```

分别修改：

```text
alias
status
progress
primary reference
directory
```

验证其他层没有意外变化。

---

## Test R — CLI / MCP Error Propagation

```text
test_v03_protocol_error_propagation
```

至少覆盖：

```text
NotFound
Validation
Conflict
```

不得返回伪成功。

---

# 29. Required Test Matrix

最终至少达到：

```text
Existing v0.2 tests
        +
Test I
Test J
Test K
Test L
Test M
Test N
Test O
Test P
Test Q
Test R
```

即：

```text
v0.2 baseline
+
10 new integrity tests
```

所有测试：

```text
0 failed
0 ignored
0 warnings
```

---

# 30. Required Build Verification

必须执行：

```bash
cargo test --workspace
```

并确认：

```text
Core       PASS
Database   PASS
Scanner    PASS
CLI        PASS
MCP        PASS
```

Desktop：

```bash
pnpm --filter @looma/desktop build
```

必须：

```text
TypeScript 0 errors
Vite 0 errors
```

如存在 Tauri build：

```bash
pnpm tauri build
```

也必须成功。

---

# 31. Required Static Architecture Audit

执行代码搜索确认不存在：

```text
AnimeTable
MangaTable
BookTable
MovieTable
GameTable

AnimeService
MangaService
BookService
MovieService
GameService

AnimeProgress
MangaProgress
BookProgress
MovieProgress
GameProgress

AnimeAsset
MangaAsset
BookAsset
MovieAsset
GameAsset
```

同时检查：

```text
GUI → SQLite
CLI → SQLite
MCP → SQLite
```

均不得存在私有绕路。

---

# 32. Data Integrity Acceptance Criteria

最终必须满足：

```text
Work
 ├── Identity
 ├── Metadata
 ├── Lifecycle
 ├── Progress
 └── Connections
```

任意单层操作：

```text
不会破坏其他层
```

---

# 33. Final Acceptance Matrix

| # | Acceptance Item | Required |
|---|---|---|
| 1 | Generic Work Architecture preserved | PASS |
| 2 | Zero media-specific tables | PASS |
| 3 | Zero media-specific services | PASS |
| 4 | Relocate atomic transaction | PASS |
| 5 | Relocate rollback | PASS |
| 6 | Work identity preserved | PASS |
| 7 | Primary Reference unique | PASS |
| 8 | Cross-entity primary rejected | PASS |
| 9 | Primary creation replacement atomic | PASS |
| 10 | Alias semantic search | PASS |
| 11 | Alias deduplication | PASS |
| 12 | Universe relation integrity | PASS |
| 13 | Self relation rejected | PASS |
| 14 | Duplicate relation protected | PASS |
| 15 | Cross-layer isolation | PASS |
| 16 | CLI errors propagated | PASS |
| 17 | MCP errors propagated | PASS |
| 18 | Old data readable | PASS |
| 19 | New enum compatibility explicitly tested/documented | PASS |
| 20 | Cold restart persistence | PASS |
| 21 | Game regression suite | PASS |
| 22 | Desktop build | PASS |

---

# 34. Definition of Done

只有满足以下全部条件，才能声明：

```text
Looma Work Model Semantic Hardening v0.3
PASS
```

### Mandatory

```text
cargo test --workspace
    ↓
0 failed

pnpm desktop build
    ↓
0 errors
```

### Mandatory Integrity

```text
Relocate atomic
Primary unique
Alias semantic
Relation valid
Cross-layer isolated
Protocol errors propagated
```

### Mandatory Compatibility

```text
Old database
    ↓
New application
    ↓
Successfully loaded
```

如果没有真实验证：

```text
Old binary
    ↓
New database
```

则不得声称：

```text
100% backward binary compatibility
```

只能声明：

```text
Data backward compatibility verified
```

---

# 35. Explicit Non-Goals

本 Patch 不实现：

```text
全文搜索引擎
SQLite FTS5
自动在线元数据抓取
自动识别 Anime / Manga / Book
自动合并 Work
自动选择 Primary Reference
推荐系统
知识图谱
云同步
```

这些属于后续能力。

---

# 36. Architecture Freeze Rule

完成 v0.3 后：

> **不要因为业务类型增加而继续扩张 Schema。**

未来新增：

```text
Anime
Manga
Book
Movie
TV
Music
Game
Novel
Comic
Podcast
```

原则仍然是：

```text
Entity
  +
WorkMetadata
  +
TypeSpecificMetadata
  +
WorkProgress
  +
Connections
```

而不是：

```text
One media type
    ↓
One table
    ↓
One service
```

---

# 37. Final Design Principle

Looma 的 Work Model 应保持：

```text
             ┌──────────────┐
             │    Entity    │
             └──────┬───────┘
                    │
          ┌─────────┴─────────┐
          │                   │
    Work Identity        Work Metadata
          │                   │
          └─────────┬─────────┘
                    │
             Lifecycle
                    │
               Progress
                    │
        ┌───────────┼───────────┐
        │           │           │
      Asset      Reference    Memory
        │           │           │
        └───────────┼───────────┘
                    │
               Relations
                    │
          Other Work Entities
```

核心原则：

> **Work 是一个稳定的实体身份。**
>
> **Metadata 描述“它是什么”。**
>
> **Lifecycle 描述“我如何管理它”。**
>
> **Progress 描述“我消费到哪里”。**
>
> **Connections 描述“它与现实世界发生了什么连接”。**
>
> **Relation 描述“它与其他实体是什么关系”。**

任何未来功能都不得破坏这五层边界。

---

# 38. Audit Output Requirement

实施完成后，不要只返回：

```text
15 / 15 PASS
```

必须生成新的：

```text
Looma Work Model Semantic Hardening v0.3
Implementation Audit & Acceptance Review Report
```

至少包含：

1. 实际修改文件；
2. 实际新增 API；
3. Relocate 事务实现证据；
4. Primary 唯一性实现证据；
5. Alias 检索实现证据；
6. Relation 完整性实现证据；
7. 所有新增测试名称；
8. `cargo test --workspace` 原始结果；
9. Desktop build 原始结果；
10. Git commit；
11. 是否存在 warnings；
12. 是否存在未提交文件；
13. 数据兼容性测试结论。

禁止只依据“代码看起来正确”宣布 PASS。

必须以：

```text
Implementation
+
Automated Tests
+
Build Verification
+
Architecture Audit
```

四项共同作为最终验收依据。

---

# 39. Final Acceptance Statement

只有在上述要求全部满足后，才允许将状态标记为：

```text
Looma Work Model Semantic Hardening v0.3
────────────────────────────────────────
STATUS: PASS
```

否则必须明确：

```text
PASS
PARTIAL
FAIL
```

不得使用：

```text
READY FOR ACCEPTANCE
```

替代实际测试结论。

**v0.3 的目标不是增加更多 Work 功能，而是让已经存在的 Generic Work Model 在异常、失败、重定位、主入口切换、检索和跨层更新情况下仍然保持数据一致性。**