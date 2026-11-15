# Looma Work Model Semantic Hardening v0.4
## Relation Safety & Concurrency Integrity Patch

> **Patch Type**: P0/P1 Consistency & Data Integrity Hardening  
> **Predecessor**: `Looma Work Model Semantic Hardening v0.3`  
> **Scope**: `crates/core`, `crates/database`, `crates/mcp`, `crates/cli`, `apps/desktop`  
> **Primary Goal**: 修复 v0.3 验收中仍存在的关系删除过宽、关系方向语义不严谨、Primary 唯一性缺少数据库级约束、Relocate 幂等边界不足等问题。
>
> **重要原则**：
>
> - 不新增 Anime / Manga / Book / Game 等专属表
> - 不新增媒体专属 Service
> - 不新增 Work 类型专属数据库结构
> - 不改变既有 Work Identity
> - 不破坏 v0.1 / v0.2 / v0.3 已通过测试
> - 优先修复数据完整性，不为了测试 PASS 而弱化语义

---

# 1. 本轮审计发现

v0.3 已经解决了大量问题，但审阅当前实现后，仍发现以下风险。

| 优先级 | 问题 | 风险 |
|---|---|---|
| **P0** | `relocate_work_directory` 删除关系条件未限定 `relation_type` | 可能误删 Work 与旧 Asset 之间的其他合法关系 |
| **P0** | Relocate 对关系方向的语义不够严格 | 可能把反向关系误认为目录挂载关系 |
| **P1** | Primary Reference 唯一性主要依赖事务代码，而非 SQLite Schema 级约束 | 长期维护 / 并发场景缺少最终数据库不变量 |
| **P1** | Relocate 对“目标目录已经被当前 Work 关联”的幂等语义需要明确 | 可能产生重复关系或不必要的关系重建 |
| **P1** | `create_relation` 的重复判断与 Relocate 的关系判断规则不完全统一 | 容易出现同一关系在不同入口下语义不一致 |
| **P2** | v0.3 的“22/22 PASS”存在测试覆盖与实际不变量之间的差距 | 测试通过不等于所有数据完整性边界已覆盖 |
| **P2** | `#[serde(other)]` 的兼容性措辞需要更精确 | 当前文档把“新 Reader 读取未知枚举值”和“旧 Binary 读取新数据”概念混淆 |

本补丁必须解决上述问题。

---

# 2. P0 — Relocate 严禁删除非 Directory Relation

## 2.1 当前风险

当前实现中存在类似：

```sql
DELETE FROM relations
WHERE
    (source_id = ?1 AND target_id = ?2)
    OR
    (source_id = ?2 AND target_id = ?1)
```

这个条件存在严重问题：

> 它只按照 `source_id + target_id` 删除关系，却没有限制 `relation_type`。

假设：

```text
Work A
  │
  ├── attaches ──> Directory Asset X
  │
  └── references ──> Asset X
```

执行：

```text
relocate_work_directory(A, X, Y)
```

理论上只应该删除：

```text
A --attaches--> X
```

但当前 SQL 可能把：

```text
A --references--> X
```

也一起删除。

这是 **P0 数据完整性问题**。

---

# 3. P0 — Relocate 必须严格定义 Directory Relation

## 3.1 标准关系语义

Work 的本地目录挂载必须使用明确关系类型：

```rust
relation_types::ATTACHES
```

或者项目现有对应的目录挂载关系常量。

不要通过：

```text
source_id + target_id
```

推断“这是目录关系”。

必须同时满足：

```text
source_id == work_id
target_id == old_asset_id
relation_type == ATTACHES
```

---

## 3.2 正确删除条件

将当前：

```sql
DELETE FROM relations
WHERE
    (source_id = ?1 AND target_id = ?2)
    OR
    (source_id = ?2 AND target_id = ?1)
```

修改为严格的：

```sql
DELETE FROM relations
WHERE
    source_id = ?1
    AND target_id = ?2
    AND relation_type = ?3
```

参数：

```text
work_id
old_asset_id
ATTACHES
```

禁止在 Directory Relocate 中使用“双向删除”。

---

# 4. P0 — Relocate 必须验证旧关系确实是 ATTACHES

当前：

```sql
SELECT COUNT(*)
FROM relations
WHERE
    (
        source_id = ?1 AND target_id = ?2
    )
    OR
    (
        source_id = ?2 AND target_id = ?1
    )
```

必须修改为：

```sql
SELECT COUNT(*)
FROM relations
WHERE
    source_id = ?1
    AND target_id = ?2
    AND relation_type = ?3
```

即：

```text
Work → Asset
relation_type = attaches
```

只有满足这个条件才允许执行 Relocate。

如果：

```text
Work A
  └── references → Asset X
```

但不存在：

```text
Work A
  └── attaches → Asset X
```

那么：

```text
relocate_work_directory(A, X, Y)
```

必须返回：

```text
Validation
```

不得把 `references` 当成目录挂载。

---

# 5. P0 — Relocate 必须验证 Asset 类型

除了验证 Asset 存在，还必须验证：

```text
old_asset.kind == Directory
```

否则：

```text
Work → File Asset
```

不能被当作目录进行：

```text
relocate_work_directory
```

返回：

```text
LoomaError::Validation
```

错误信息应该明确：

```text
Asset <id> is not a directory asset
```

同样，新路径生成的 Asset 必须保证：

```text
kind == Directory
```

---

# 6. P0 — 新关系必须明确为 ATTACHES

Relocate 创建的新关系不得依赖调用方随意传入 relation type。

Database 层必须保证：

```text
relation_type = ATTACHES
source_id = work_id
target_id = new_asset.id
```

推荐让 DB API 接受：

```rust
work_id
old_asset_id
new_asset
```

然后内部构造关系。

而不是完全信任：

```rust
new_relation: &Relation
```

如果为了保持现有 API 不变，也必须在 DB 层验证：

```rust
new_relation.source_id == work_id
new_relation.target_id == new_asset.id
new_relation.relation_type == ATTACHES
```

否则返回 Validation。

---

# 7. P1 — Relocate 的幂等语义必须明确

以下场景必须定义清楚：

```text
Work A
  └── attaches → Directory X
```

调用：

```text
relocate(A, X, X)
```

应该：

```text
PASS / no-op
```

不能：

```text
delete X relation
create X relation
```

更不能产生重复 Relation。

---

## 7.1 已经存在目标目录的情况

如果：

```text
Work A
  ├── attaches → Directory X
  └── attaches → Directory Y
```

执行：

```text
relocate(A, X, Y)
```

目标 `Y` 已经属于同一个 Work。

必须定义为：

```text
最终只保留：
A → Y

A → X 被删除
```

不能产生两个：

```text
A → Y
A → Y
```

因此 Relocate 在事务中必须检查：

```sql
SELECT id
FROM relations
WHERE
    source_id = work_id
    AND target_id = new_asset_id
    AND relation_type = ATTACHES
```

如果已经存在：

```text
删除 old relation
保留 existing new relation
```

而不是再次 INSERT。

---

# 8. P1 — create_relation 与 Relocate 的关系语义统一

统一关系判断规则：

```text
Relation Identity =
    source_id
    + target_id
    + relation_type
```

不得出现：

```text
create_relation:
    source + target + type

relocate:
    source + target
```

这种不同语义。

---

## 8.1 重复关系必须保持幂等

例如：

```text
A --adaptation_of--> B
```

再次创建：

```text
A --adaptation_of--> B
```

必须：

```text
OK
```

但数据库中仍然只有一条。

---

## 8.2 反向关系不是重复关系

例如：

```text
A --sequel_of--> B
B --prequel_of--> A
```

这是两个合法关系。

不能简单通过：

```text
unordered(source, target)
```

去重。

关系是否允许反向存在，必须由：

```text
source_id
target_id
relation_type
```

共同决定。

因此：

```text
A → B
```

与：

```text
B → A
```

不是天然重复。

---

# 9. P1 — Primary Reference 增加数据库级唯一不变量

当前 v0.3 已经通过事务代码保证：

```text
同一 Entity 最多一个 Primary
```

但长期来看，仅依赖业务代码是不够的。

SQLite 数据库本身应该拥有最终不变量。

如果当前 External Reference 存储结构允许，应增加唯一部分索引：

```sql
CREATE UNIQUE INDEX IF NOT EXISTS
idx_external_references_one_primary_per_entity
ON external_references(entity_id)
WHERE is_primary = 1;
```

注意：

> 如果当前 `is_primary` 是 JSON metadata，而不是独立列，则不要强行改变现有 Schema。

此时必须在实现中明确：

```text
Schema-level unique constraint unavailable
```

并确保所有写入口都严格通过统一事务方法。

不要为了“看起来更完整”而引入不必要 Migration。

---

# 10. P1 — Primary Reference 所有写入口统一

以下入口必须最终统一走：

```rust
set_primary_external_reference(...)
```

包括：

```text
create_external_reference(is_primary=true)
CLI ref add --primary
CLI ref primary
MCP create_external_reference(is_primary=true)
MCP set_primary_external_reference
GUI 设置主入口
```

禁止任意入口自行：

```sql
UPDATE external_references
SET is_primary = ...
```

---

# 11. P1 — Primary Reference 删除后的语义冻结

删除当前 Primary 后：

```text
remaining references:
    is_primary = false
```

不要自动选择：

```text
next reference
```

这一点 v0.3 已经定义正确。

本轮需要增加明确测试：

```text
Primary A
Secondary B
Secondary C

delete A

assert:
A does not exist
B.is_primary == false
C.is_primary == false
```

---

# 12. P1 — Search API 语义冻结

`search_works` 必须明确：

SQLite：

```text
title LIKE
properties_json LIKE
```

只是：

```text
candidate pre-filter
```

最终结果必须经过：

```rust
Entity::matches_query()
```

严格确认。

最终可命中的字段只允许：

```text
title
original_title
description
aliases
```

禁止因为：

```text
properties_json
```

中的其他字段出现字符串而产生最终命中。

例如：

```json
{
  "broadcast_day": "Sunday"
}
```

搜索：

```text
Sunday
```

不能命中该作品。

---

# 13. P2 — Alias Canonicalization 明确规则

`set_aliases()` 必须保持：

```text
trim
case-insensitive deduplication
empty string removal
```

例如输入：

```text
[
    "AOT",
    " aot ",
    "Aot",
    "",
    "進撃の巨人"
]
```

结果：

```text
[
    "AOT",
    "進撃の巨人"
]
```

同时注意：

> 去重时大小写不敏感，不代表必须破坏用户原始展示大小写。

建议保留第一次出现的 canonical display value。

---

# 14. P2 — Compatibility 文档修正

v0.3 中：

```rust
#[serde(other)]
```

对应的准确语义应该表述为：

> 新版本 Reader 可以安全读取包含未知枚举值的数据，并将其映射到 `Other` / `Unknown`。

这属于：

```text
Forward-reading compatibility
```

而不是：

```text
Old Binary compatibility
```

必须明确区分：

### A. Old Data → New Binary

必须 PASS。

例如：

```text
旧数据库缺少 aliases
旧数据库缺少 type_metadata
```

新版本能够正常读取。

---

### B. New Data → Old Binary

不保证。

尤其：

```text
new aliases
new status
new type_metadata
```

旧版本可能：

```text
忽略未知 JSON 字段
```

但无法理解新的业务语义。

因此文档继续保持：

```text
NOT GUARANTEED
```

---

# 15. 新增测试矩阵

本轮至少新增以下测试。

## Test S — Relocate Relation-Type Safety

验证：

```text
Work A
  attaches → Asset X
  references → Asset X
```

执行：

```text
relocate(A, X, Y)
```

最终：

```text
A --attaches--> Y
A --references--> X
```

必须保持。

---

## Test T — Relocate Direction Safety

验证：

```text
Asset X
  attaches → Work A
```

不存在：

```text
Work A
  attaches → Asset X
```

执行 Relocate：

```text
relocate(A, X, Y)
```

必须：

```text
Validation
```

不得成功。

---

## Test U — Relocate Asset Kind Validation

验证：

```text
Asset X.kind = File
```

调用：

```text
relocate_work_directory(A, X, Y)
```

必须：

```text
Validation
```

---

## Test V — Relocate Existing Target Idempotency

初始：

```text
A → X
A → Y
```

执行：

```text
relocate(A, X, Y)
```

最终：

```text
A → Y
```

且：

```text
count(A, Y, attaches) == 1
```

---

## Test W — Relocate Same Path Idempotency

执行：

```text
relocate(A, X, X)
```

必须：

```text
PASS
```

且：

```text
count(A, X, attaches) == 1
```

---

## Test X — Relation Direction Independence

验证：

```text
A → B
B → A
```

在允许的不同 relation_type 下不会被错误当成重复关系。

---

## Test Y — Primary Delete No Auto Promotion

验证：

```text
A primary
B secondary
C secondary
```

删除 A 后：

```text
B = false
C = false
```

---

## Test Z — Primary Write Path Consistency

分别通过：

```text
Core
CLI
MCP
```

设置 Primary。

最终数据库必须始终满足：

```text
count(primary references for entity) <= 1
```

---

# 16. 回归测试要求

执行：

```bash
cargo test --workspace
```

必须：

```text
0 failed
0 ignored
0 warnings
```

并且：

```text
v0.1 Game tests
v0.2 semantic tests
v0.3 consistency tests
v0.4 integrity tests
```

全部保留。

不得删除旧测试来降低测试数量。

---

# 17. GUI / CLI / MCP 回归要求

必须验证三端仍然只调用：

```text
LoomaCore
```

禁止：

```text
GUI → SQLite
CLI → SQLite
MCP → SQLite
```

所有：

```text
relocate
primary
search
relation
```

都必须通过统一 Core Facade。

---

# 18. 最终验收标准

本补丁完成后，最终状态必须满足：

```text
[P0] Relocate 只删除 ATTACHES relation
[P0] Relocate 严格验证 Work → Directory Asset
[P0] Relocate 不误删其他 Relation
[P0] Relocate 新关系只能是 ATTACHES
[P0] Relocate 失败完整 rollback

[P1] Existing target 不产生重复 Relation
[P1] Same-path relocate 幂等
[P1] Relation direction semantics 明确
[P1] Primary Reference 全部写入口统一
[P1] Primary 最多一个
[P1] Primary 删除不自动晋升

[P2] Alias canonicalization 稳定
[P2] Search semantic filtering 稳定
[P2] Compatibility terminology 准确

[REGRESSION] v0.1 / v0.2 / v0.3 全部保持 PASS
```

---

# 19. 严禁事项

本轮不得：

```text
❌ 新增 GameTable
❌ 新增 AnimeTable
❌ 新增 MangaTable
❌ 新增 BookTable
❌ 新增 MovieTable

❌ 新增 GameService
❌ 新增 AnimeService
❌ 新增 MangaService

❌ 为 Work 类型增加专属数据库表
❌ GUI 直接访问 SQLite
❌ CLI 绕过 LoomaCore
❌ MCP 绕过 LoomaCore

❌ 为了测试通过删除已有测试
❌ 修改测试使错误行为变成“合法行为”
❌ 用双向 source/target 删除替代明确 relation_type
```

---

# 20. 完成后必须提交的 Audit Report

反重力完成后必须输出：

```text
Looma Work Model Semantic Hardening v0.4
Implementation Audit & Acceptance Review Report
```

至少包含：

1. 实际修改文件清单
2. P0/P1/P2 修改说明
3. Relocate 最终 SQL / 事务边界
4. Relation type safety 证据
5. Asset kind validation 证据
6. Primary Reference 唯一性证据
7. 新增 Test S-Z 的执行结果
8. 全量 `cargo test --workspace`
9. Desktop build
10. Git commit
11. Git status
12. Compatibility statement
13. 最终 PASS / PARTIAL / FAIL

---

# 21. 最终目标

v0.4 不增加新的业务能力。

本轮唯一目标：

> **把 v0.3 已经建立的 Generic Work Architecture，从“业务逻辑正确”进一步提升到“关系级数据完整性正确”。**

尤其确保：

```text
Relocate
    ↓
只改变 Work 的 Directory Attachment
    ↓
不影响其他 Relation
    ↓
不改变 Work Identity
    ↓
失败完整 rollback
```

以及：

```text
Primary Reference
    ↓
同一 Entity
    ↓
最多一个 Primary
    ↓
所有入口统一
    ↓
删除不自动晋升
```

完成上述条件后，才建议将：

```text
Looma Work Model Semantic Hardening v0.4
```

标记为：

```text
FROZEN / ACCEPTED
```