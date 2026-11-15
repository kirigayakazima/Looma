# Looma Work Model Semantic Hardening v0.6
## Database Invariant & Recovery Integrity Patch

> **Patch Version**: v0.6  
> **Patch Target**: `Looma Work Model Semantic Hardening v0.6 — Database Invariant & Recovery Integrity Patch`  
> **Repository Baseline**: v0.5 `ACCEPTED / PASS`  
> **Architecture Principle**: Generic Work Architecture 零媒体专属表、零媒体专属 Service、零业务专属 SQLite 表、保持现有通用 Schema；进一步将核心不变量从“写入口约束”提升为“数据库级可验证约束 + 事务级一致性 + 可诊断/可修复完整性”。

---

# 1. v0.6 目标

v0.5 已经完成：

- Relation logical identity；
- Primary Reference 写入口统一；
- Relocate 原子事务；
- Work → Directory Attachments 严格方向与类型校验；
- Core Integrity Checker；
- CLI / MCP Integrity Check；
- 单进程并发回归；
- Cold Restart；
- Cross-layer isolation。

v0.6 不再继续增加新的业务模型，而是针对当前架构剩余的完整性风险进行最后一轮基础设施级加固：

### P0

1. **Primary Reference 唯一性必须从“写入口保证”提升为数据库层可证明的不变量。**
2. **Work → Directory ATTACHES 唯一性必须从 Repository 逻辑保证提升为数据库层可证明的不变量。**
3. 所有 Integrity Check 必须能够区分：
   - 数据损坏；
   - API 使用错误；
   - 数据库约束冲突；
   - 并发竞争失败。
4. 并发写入失败不得产生半完成状态。

### P1

5. 增加 Integrity Repair / Recovery Preview 能力，但必须默认只读。
6. Integrity Report 增加稳定 machine-readable violation code。
7. Integrity Check 覆盖 Repository 直接写入路径，而不仅仅覆盖 Facade。
8. 明确单进程并发与多进程并发的保证边界。

### P2

9. 增加数据库 invariant introspection。
10. 完善 migration / upgrade 后完整性验证。
11. 明确旧数据存在重复关系或重复 Primary 时的处理策略：**检测优先，自动修复必须显式执行。**

---

# 2. 强制架构约束

本补丁不得引入：

- `anime_table`
- `manga_table`
- `book_table`
- `game_table`
- `AnimeService`
- `MangaService`
- `BookService`
- `GameService`
- `WorkIntegrityService`
- 任意媒体专属 SQLite 表
- 任意新的业务专属持久化 Schema

允许：

- `LoomaCore`
- 现有 `EntityService`
- 现有 `RelationService`
- 现有 `ExternalReferenceService`
- 现有 `LoomaDb`
- 通用 `IntegrityReport`
- 通用 `IntegrityViolation`

**核心原则：Integrity 是 Core / Database 的横切能力，不得演化成新的业务 Service。**

---

# 3. 当前 v0.5 架构中需要继续收口的问题

## 3.1 Primary Reference

当前保证主要来自：

```text
create_external_reference()
        ↓
set_primary_external_reference()
        ↓
SQLite Transaction
```

这能够保证正常 API 调用下的唯一性。

但是：

```text
Database invariant
        ≠
Facade invariant
        ≠
Repository write-path invariant
```

因此 v0.6 必须明确：

> “所有正常写入口最终都会维持不变量”与“数据库自身能够拒绝非法状态”是两个不同层级的保证。

---

## 3.2 Attachment

同样：

```text
Work -> ATTACHES -> Directory
```

目前通过 Repository / Facade 逻辑避免重复。

但如果未来存在：

- 新的 Repository 调用；
- migration；
- import；
- recovery；
- MCP 新入口；
- CLI 新入口；
- 第三方数据库操作；

则单纯依赖 Facade 逻辑容易出现重复关系。

因此 v0.6 必须把：

```text
(source_id, relation_type, target_id)
```

的逻辑唯一性进一步明确为数据库级 invariant。

---

# 4. Relation Logical Identity 最终规范

Relation 的逻辑身份定义为：

```text
(source_id, relation_type, target_id)
```

方向必须参与身份判定。

因此：

```text
A -> sequel_of -> B
```

与：

```text
B -> sequel_of -> A
```

是两个完全不同的 Relation。

同样：

```text
A -> sequel_of -> B
A -> prequel_of -> B
```

也是两个不同 Relation。

禁止使用：

```text
min(source_id, target_id)
max(source_id, target_id)
```

等无方向 canonicalization。

---

# 5. Relation 唯一性 Database Invariant

## 5.1 P0

必须保证：

```text
(source_id, relation_type, target_id)
```

最多存在一条。

理想实现优先级：

```text
Database Constraint
        ↓
Repository Idempotency
        ↓
Facade Validation
        ↓
MCP / CLI
```

而不是反过来。

---

## 5.2 如果当前 Schema 不允许直接新增 UNIQUE Constraint

由于本项目当前明确要求：

> Zero Schema Migration / Zero New Physical Table

如果当前 SQLite Schema 无法安全增加：

```sql
UNIQUE(source_id, relation_type, target_id)
```

则不得擅自修改物理 Schema。

此时必须：

1. 在 Repository 层统一使用事务；
2. 在同一事务中进行 existence check + insert；
3. 使用 SQLite transaction serialization 保证单连接并发安全；
4. Integrity Checker 必须能够检测历史重复；
5. 明确文档：

```text
Logical Invariant: enforced by repository transaction
Physical DB UNIQUE constraint: unavailable under frozen schema
```

**不得声称已经具备 SQLite physical UNIQUE constraint。**

---

# 6. Primary Reference 最终规范

## 6.1 Invariant

对于任意 Entity：

```text
count(is_primary = true) <= 1
```

必须始终成立。

注意：

```text
<= 1
```

而不是：

```text
== 1
```

因为：

- Entity 可以没有 ExternalReference；
- Primary 被删除后不自动晋升；
- 所有剩余 Reference 可以全部为 false。

---

## 6.2 Primary Creation

执行：

```text
create_external_reference(..., is_primary=true)
```

必须在单一 transaction 中完成：

```text
BEGIN

existing primary → false

INSERT new reference → true

COMMIT
```

禁止：

```text
INSERT
COMMIT

UPDATE old primary
COMMIT
```

这种两阶段行为。

---

## 6.3 Primary Switching

执行：

```text
set_primary_external_reference(entity_id, reference_id)
```

必须：

1. 验证 Entity；
2. 验证 Reference；
3. 验证 Reference owner；
4. 同一 transaction 清除旧 Primary；
5. 设置新 Primary；
6. Commit。

---

## 6.4 Primary Deletion

删除 Primary：

```text
primary → deleted
remaining references → all false
```

禁止隐式：

```text
remaining[0] → primary
```

---

# 7. IntegrityViolation 稳定错误码

v0.5 的：

```rust
IntegrityViolation
```

必须进一步拥有稳定 machine-readable code。

建议：

```rust
pub enum IntegrityViolationCode {
    DuplicatePrimary,
    DuplicateRelation,
    InvalidAttachmentDirection,
    InvalidAttachmentKind,
    SelfRelation,
    MissingRelationEndpoint,
    DanglingAsset,
    DanglingReference,
    InvalidWorkMetadata,
}
```

每个 violation 至少包含：

```rust
pub struct IntegrityViolation {
    pub code: IntegrityViolationCode,
    pub message: String,
    pub entity_id: Option<String>,
    pub relation_id: Option<String>,
    pub reference_id: Option<String>,
}
```

要求：

- CLI 使用 `code`；
- MCP 输出 `code`；
- UI 可以根据 `code` 显示；
- 人类可读 `message` 不得成为程序逻辑判断依据。

---

# 8. IntegrityReport 扩展

建议：

```rust
pub struct IntegrityReport {
    pub valid: bool,
    pub violations: Vec<IntegrityViolation>,
    pub checked_relations: usize,
    pub checked_references: usize,
    pub checked_assets: usize,
}
```

这样：

```json
{
  "valid": true,
  "violations": [],
  "checked_relations": 12,
  "checked_references": 3,
  "checked_assets": 2
}
```

可以明确告诉调用方：

> 检查了什么，而不是仅仅告诉它 PASS。

---

# 9. Integrity Check 必须检查的内容

`validate_work_integrity(work_id)` 至少检查：

## A. Work Identity

- Work 存在；
- Work metadata 可反序列化；
- Work type 合法；
- Record status 合法。

## B. Primary Reference

- Primary 数量 <= 1；
- 每个 Reference owner 正确；
- Reference 不得指向不存在 Entity。

## C. Relations

- Relation source 存在；
- Relation target 存在；
- source != target；
- Relation direction 合法；
- 同一 logical identity 不得重复。

## D. Attachments

对于：

```text
relation_type == attaches
```

必须满足：

```text
source == Work
target == Asset
target.kind == directory
```

## E. Cross-layer consistency

验证：

```text
Identity
Metadata
Lifecycle
Progress
Connections
```

之间不存在语义污染。

---

# 10. Strict Attachment Query

保留：

```rust
pub fn list_work_directory_attachments(
    &self,
    work_id: &str,
) -> LoomaResult<Vec<(Asset, Relation)>>
```

其语义必须严格定义为：

```text
Work
  |
  +-- attaches --> Directory Asset
```

不得返回：

```text
Work -> references -> Directory
Work -> attaches -> Document
Asset -> attaches -> Work
Work -> attaches -> Memory
```

---

# 11. Relocate 最终事务规范

`relocate_work_directory` 必须保持单一事务：

```text
BEGIN
    Validate Work
    Validate old Asset
    Validate old Asset.kind == directory
    Validate old relation
    Validate old relation direction
    Validate old relation type == attaches

    Validate new Asset.kind == directory

    Upsert new Asset

    Delete ONLY:
        Work -> attaches -> old Asset

    Check:
        Work -> attaches -> new Asset

    If missing:
        INSERT new attaches

COMMIT
```

禁止：

```text
delete old
commit
insert new
commit
```

---

# 12. Relocate Failure Rule

任意步骤失败：

```text
ROLLBACK
```

必须满足：

```text
Old Attachment == unchanged
New Attachment == unchanged
Work == unchanged
Metadata == unchanged
Progress == unchanged
Status == unchanged
Primary References == unchanged
```

失败审计记录必须包含：

```text
operation
work_id
old_asset_id
new_asset_id
failure_code
failure_message
```

---

# 13. 并发语义重新定义

v0.5 中：

```text
Arc<Mutex<Connection>>
```

能够保证：

> 单进程内共享 Connection 的访问被串行化。

但不能宣称：

> 全系统 / 全机器 / 多进程数据库级全局并发锁。

因此 v0.6 必须在代码注释、文档和测试中明确：

### Guaranteed

```text
same process
same LoomaDb connection
multiple threads
→ serialized transaction access
```

### Not Guaranteed Unless Explicitly Tested

```text
multiple Looma processes
multiple database connections
external SQLite writers
network filesystem concurrent access
```

不得把：

```text
Arc<Mutex<Connection>>
```

描述成通用数据库级并发机制。

---

# 14. 多进程边界测试

如果当前测试环境允许，新增：

```text
test_v06_multi_process_database_contention
```

如果测试框架无法可靠执行多进程 SQLite contention，则：

**不要伪造 PASS。**

改为：

```text
NOT IMPLEMENTED / OUT OF CURRENT GUARANTEE
```

并在最终报告明确说明。

---

# 15. Integrity Repair Preview

增加只读 API：

```rust
pub fn preview_work_integrity_repair(
    &self,
    work_id: &str,
) -> LoomaResult<IntegrityRepairPlan>;
```

其中：

```rust
pub struct IntegrityRepairPlan {
    pub safe: bool,
    pub actions: Vec<IntegrityRepairAction>,
}
```

例如：

```text
Duplicate Primary
        ↓
Keep: explicitly selected / deterministic reference
Demote: others
```

但：

> Preview 不得修改数据库。

---

# 16. Integrity Repair 执行原则

v0.6 **默认不提供自动 Repair 写入口**。

原因：

```text
检测损坏
    ≠
知道正确答案
```

例如：

```text
Primary A
Primary B
```

数据库可以确定：

```text
违反 <= 1
```

但无法仅凭数据库判断：

```text
A 才是用户真正想要的 Primary
```

因此：

```text
v0.6 = Detect + Preview
```

而不是：

```text
v0.6 = Auto Repair
```

如果未来增加 Repair：

必须：

- 显式用户确认；
- 明确 repair plan；
- 单一 transaction；
- 产生 audit；
- 可回滚；
- 不得静默执行。

---

# 17. CLI

保留：

```bash
looma work check <WORK_ID>
```

扩展输出：

### PASS

```text
Work Integrity: PASS

Checked:
  Relations: 12
  References: 3
  Assets: 2

No violations found.
```

### FAIL

```text
Work Integrity: FAIL

[duplicate_primary]
Entity has more than one primary external reference.

[invalid_attachment_kind]
ATTACHES target is not a directory asset.

Exit code: 1
```

---

# 18. MCP

保留：

```text
check_work_integrity
```

输出建议：

```json
{
  "valid": false,
  "violations": [
    {
      "code": "duplicate_primary",
      "message": "Entity has more than one primary external reference.",
      "entity_id": "work_123"
    }
  ],
  "checked_relations": 8,
  "checked_references": 3,
  "checked_assets": 1
}
```

权限必须保持：

```text
ReadOnly
```

---

# 19. 新增测试：AP–AZ

## AP

`test_v06_repository_duplicate_relation_invariant`

目标：

Repository 直接重复写入同一：

```text
source + relation_type + target
```

不会产生第二条。

---

## AQ

`test_v06_integrity_violation_machine_codes`

目标：

所有 IntegrityViolation 都具有稳定 machine-readable code。

---

## AR

`test_v06_primary_zero_or_one_invariant`

验证：

```text
0 primary → valid
1 primary → valid
2 primary → invalid
```

---

## AS

`test_v06_primary_creation_transactional`

验证：

Primary 创建失败时：

```text
old primary unchanged
new reference not partially inserted
```

---

## AT

`test_v06_relocate_failure_preserves_all_layers`

Relocate 任意失败后验证：

```text
Identity
Metadata
Status
Progress
References
Relations
Assets
```

全部保持不变。

---

## AU

`test_v06_attachment_integrity_machine_codes`

验证：

```text
reverse attachment
document attachment
missing target
duplicate attachment
```

均返回稳定 violation code。

---

## AV

`test_v06_integrity_report_statistics`

验证：

```text
checked_relations
checked_references
checked_assets
```

统计值正确。

---

## AW

`test_v06_integrity_preview_is_read_only`

执行：

```text
preview_work_integrity_repair()
```

前后数据库 hash / snapshot 完全一致。

---

## AX

`test_v06_duplicate_primary_detected_after_direct_repository_corruption`

模拟历史数据损坏：

```text
Primary A
Primary B
```

绕过正常 API 写入后：

```text
validate_work_integrity()
```

必须检测：

```text
duplicate_primary
```

---

## AY

`test_v06_duplicate_relation_detected_after_direct_repository_corruption`

模拟重复 Relation 后：

```text
validate_work_integrity()
```

必须检测：

```text
duplicate_relation
```

---

## AZ

`test_v06_cross_layer_recovery_preview_consistency`

验证 Integrity Preview 对：

```text
Relations
References
Assets
Work Metadata
```

生成稳定、可解释的 repair plan。

---

# 20. Regression Requirements

v0.1–v0.5 所有测试必须继续通过。

特别是：

```text
Work Layer Isolation
Status / Progress Independence
Alias Search
Primary Reference
Universe Relations
Relocate Atomicity
Relocate Direction Safety
Asset Kind Safety
Existing Target Idempotency
Cold Restart
Cross-layer Consistency
Concurrent Primary
Concurrent Relocate
Integrity Check
CLI
MCP
```

不得因为 v0.6 引入底层 invariant 机制而改变既有语义。

---

# 21. 不允许的“伪增强”

反重力实施时禁止为了让测试 PASS 而：

### 禁止 1

在测试里绕过真实 Repository。

### 禁止 2

只检查最终状态，不检查中间失败后的 rollback。

### 禁止 3

把：

```text
Mutex
```

描述成：

```text
database-global lock
```

### 禁止 4

把：

```text
Repository logical uniqueness
```

描述成：

```text
SQLite UNIQUE constraint
```

除非实际存在 SQLite Schema Constraint。

### 禁止 5

Integrity Checker 只检查正常 API 创建的数据。

必须能够发现：

```text
历史损坏
直接 Repository 注入
异常迁移
```

造成的非法状态。

### 禁止 6

自动选择新的 Primary 作为“修复”。

必须保持：

```text
detect first
preview second
explicit repair later
```

---

# 22. 最终验收矩阵

| # | 验收项 | Priority |
|---|---|---|
| 1 | Relation logical identity 明确 | P0 |
| 2 | Repository duplicate relation 幂等 | P0 |
| 3 | Integrity 可检测 duplicate relation | P0 |
| 4 | Primary count <= 1 | P0 |
| 5 | Primary transactional creation | P0 |
| 6 | Primary switching atomic | P0 |
| 7 | Attachment Work → Directory 严格约束 | P0 |
| 8 | Relocate atomic rollback | P0 |
| 9 | Relocate failure preserves all layers | P0 |
| 10 | Concurrent Primary safety | P0 |
| 11 | Concurrent Relocate safety | P0 |
| 12 | Integrity violation machine codes | P1 |
| 13 | Integrity report statistics | P1 |
| 14 | Repository corruption detection | P1 |
| 15 | Integrity Repair Preview read-only | P1 |
| 16 | CLI integrity output | P1 |
| 17 | MCP integrity output | P1 |
| 18 | Concurrency guarantee boundary explicitly documented | P1 |
| 19 | Cold restart consistency | P1 |
| 20 | v0.1–v0.5 regression | REGRESSION |

---

# 23. Definition of Done

v0.6 只有在以下条件全部满足后才能标记：

```text
FROZEN / ACCEPTED
```

必须：

```text
cargo test --workspace
        ↓
ALL PASS

desktop build
        ↓
PASS

release build
        ↓
PASS

integrity corruption tests
        ↓
PASS

repository-level duplicate tests
        ↓
PASS

concurrency tests
        ↓
PASS

read-only repair preview
        ↓
PASS
```

如果某项没有真实执行：

```text
NOT VERIFIED
```

不得写：

```text
PASS
```

---

# 24. 最终架构目标

完成 v0.6 后，Work 模型完整性层应形成：

```text
                 ┌──────────────────────────┐
                 │       Desktop / CLI      │
                 │          / MCP           │
                 └────────────┬─────────────┘
                              │
                              ▼
                 ┌──────────────────────────┐
                 │        LoomaCore         │
                 │                          │
                 │ Validation / Semantics   │
                 │ Integrity / Audit        │
                 └────────────┬─────────────┘
                              │
                              ▼
                 ┌──────────────────────────┐
                 │      Repository / DB     │
                 │                          │
                 │ Transaction Atomicity    │
                 │ Logical Uniqueness       │
                 │ Concurrency Serialization│
                 └────────────┬─────────────┘
                              │
                              ▼
                 ┌──────────────────────────┐
                 │      Generic SQLite      │
                 │                          │
                 │ entities                 │
                 │ assets                   │
                 │ relations                │
                 │ external references      │
                 │ memories                 │
                 └──────────────────────────┘
```

核心原则：

```text
UI
 ↓
Protocol
 ↓
Facade
 ↓
Repository
 ↓
Transaction
 ↓
Database
```

越靠近底层：

```text
越少业务语义
越强一致性
越强原子性
```

越靠近上层：

```text
越多用户语义
越多可读错误
越多审计信息
```

---

# 25. v0.6 Final Acceptance Statement

目标状态：

```text
================================================================================

Looma Work Model Semantic Hardening v0.6

Database Invariant & Recovery Integrity Patch

STATUS: FROZEN / ACCEPTED

================================================================================
```

v0.6 不增加新的媒体模型。

不增加新的媒体表。

不增加新的媒体 Service。

不改变 Work 五层模型。

不改变既有 Work Identity。

不改变 Status / Progress 正交关系。

不改变 External Reference 主入口语义。

不改变 Universe Relation 方向语义。

本版本唯一目标：

> **让 v0.1–v0.5 建立的语义不变量，从“代码约定”进一步收口为“可验证、可诊断、可恢复、可明确声明保证边界”的基础设施级不变量。**