# Looma Work Model Semantic Hardening v0.5
## Invariant & Concurrency Integrity Patch

> **Patch Version**: v0.5  
> **Patch Type**: Architecture Hardening / Data Integrity / Concurrency Safety  
> **Previous Baseline**: `Looma Work Model Semantic Hardening v0.4`  
> **Previous Acceptance**: 39 / 39 Workspace Tests PASS  
> **Target**: Looma Generic Work Architecture  
> **Implementation Principle**: 不新增媒体专属表、不新增媒体专属 Service、不破坏现有数据模型；进一步将 v0.1–v0.4 已建立的语义约束固化为可验证的不变量。

---

# 0. 本阶段定位

v0.1–v0.4 已经完成：

- Generic Work 五层模型；
- TypeSpecificMetadata 与 Lifecycle / Progress 解耦；
- Aliases；
- ExternalReference Primary；
- Universe Relation；
- Relocate 原子事务；
- Relocate Relation Type Safety；
- Relocate Direction Safety；
- Asset Kind Safety；
- Existing Target / Same Path Idempotency；
- Cross-layer isolation；
- CLI / MCP / Core 错误透传；
- Cold Restart Persistence。

因此，**v0.5 不再增加新的 Work 类型，也不增加新的业务功能。**

本阶段唯一目标：

> **把目前主要依赖 Core / Facade / Service 代码约束的数据不变量，进一步提升为“数据库事务 + Repository + Core + 回归测试”四层一致性保证，并验证真实并发场景下不会产生重复关系、多个 Primary 或半完成 Relocate。**

---

# 1. Architecture Guardrails

本补丁必须继续严格遵守以下红线。

## 1.1 禁止新增媒体专属表

禁止：

```text
AnimeTable
MangaTable
BookTable
MovieTable
GameTable
AnimeProgressTable
...
```

所有 Work 继续存储于：

```text
entities
```

相关数据继续通过：

```text
properties_json.work
```

承载。

---

## 1.2 禁止新增媒体专属 Service

禁止：

```text
AnimeService
MangaService
BookService
MovieService
GameService
```

所有操作继续通过：

```text
LoomaCore
EntityService
RelationService
ExternalReferenceService
```

完成。

---

## 1.3 不得破坏 v0.4 已建立语义

以下行为必须保持：

```text
Relocate 只操作 Work -> Asset 的 ATTACHES
Relocate 不得删除 REFERENCES / REFERENCED_BY / 其他 Relation
Relocate 只能操作 Directory Asset
反向 Asset -> Work ATTACHES 不得被当作合法挂载
Existing Target Relocate 必须幂等
Same Path Relocate 必须幂等

Primary 最多一个
删除 Primary 不自动晋升
跨 Entity Primary 必须拒绝

Self Relation 必须拒绝
缺失 Source / Target 必须拒绝
重复 Relation 必须幂等
```

---

# 2. v0.5 Core Objective

本阶段重点建立以下六个不变量：

```text
Invariant A
一个 Work + 一个 Target Directory
最多只能存在一条 ATTACHES Relation

Invariant B
一个 Entity
最多只能存在一个 Primary ExternalReference

Invariant C
ATTACHES 必须严格满足：

Work -> Directory Asset

Invariant D
Universe Relation 必须保持方向语义：

A -> B (sequel_of)
与
B -> A (prequel_of)

是两个独立 Relation

Invariant E
Relocate 在任何失败情况下：
旧状态必须完整保留

Invariant F
并发调用结束后：
数据库最终状态必须满足上述所有 Invariant
```

---

# 3. P0 — Work Directory Attachment Uniqueness

## 3.1 问题

v0.4 已经保证：

```text
Relocate 不会重复 INSERT 已存在的 Target
```

但仍然需要进一步验证：

如果数据库已经因为历史数据、旧版本 Bug 或外部调用存在：

```text
Work A
 ├── ATTACHES -> Directory X
 ├── ATTACHES -> Directory X
 └── ATTACHES -> Directory Y
```

后续 Relocate 不应该继续制造新的重复状态。

---

## 3.2 目标

增加统一的 Relation 查询 / 修复语义：

```rust
list_work_directory_attachments(work_id)
```

只返回：

```text
source_id = work_id
relation_type = "attaches"
target_type = "asset"
target AssetKind = Directory
```

不得通过：

```text
双向查询
```

猜测 Work Attachment。

---

## 3.3 新增 Core API

建议：

```rust
LoomaCore::list_work_directory_attachments(
    &self,
    work_id: &str,
) -> LoomaResult<Vec<(Asset, Relation)>>
```

要求：

1. Work 不存在 -> `NotFound`
2. 非 Work Entity -> `Validation`
3. 只返回合法：

```text
Work -> Directory
ATTACHES
```

4. 不返回：

```text
Asset -> Work
Work -> File
Work -> Asset REFERENCES
Work -> Asset REFERENCED_BY
```

---

# 4. P0 — Primary Reference Invariant

v0.4 已经保证所有写入口最终进入统一事务。

v0.5 需要进一步验证：

> 无论 Primary 是通过 Core、CLI、MCP、创建、切换还是删除产生的最终状态，都不能出现两个 Primary。

---

## 4.1 Primary 状态定义

允许：

```text
0 Primary
```

或者：

```text
1 Primary
```

禁止：

```text
2+
```

---

## 4.2 禁止隐式晋升

删除：

```text
A = Primary
B = normal
C = normal
```

结果必须：

```text
A = deleted
B = normal
C = normal
```

不能变成：

```text
B = Primary
```

也不能变成：

```text
C = Primary
```

---

# 5. P0 — Concurrent Primary Switching

必须增加真实并发测试。

至少模拟：

```text
Entity E

Reference A
Reference B
Reference C
```

同时执行：

```text
set_primary(A)
set_primary(B)
set_primary(C)
```

最终结果允许：

```text
A = true
B = false
C = false
```

或：

```text
A = false
B = true
C = false
```

或：

```text
A = false
B = false
C = true
```

但绝对禁止：

```text
A = true
B = true
C = true
```

以及：

```text
Primary count > 1
```

---

## 5.1 测试要求

新增：

```text
test_v05_concurrent_primary_switching
```

至少：

```text
8 concurrent operations
```

重复执行：

```text
100 rounds
```

每一轮结束后检查：

```rust
primary_count <= 1
```

---

# 6. P0 — Concurrent Relocate Safety

这是本版本最重要的测试。

模拟：

```text
Work A

Directory X
Directory Y
Directory Z
```

并发执行：

```text
relocate(A, X, Y)
relocate(A, X, Z)
```

以及：

```text
relocate(A, X, Y)
relocate(A, X, Y)
```

---

## 6.1 最终允许结果

并发操作结束后：

```text
ATTACHES(A, X) == 0
```

并且：

```text
count(ATTACHES(A, Y)) <= 1
count(ATTACHES(A, Z)) <= 1
```

不得出现：

```text
duplicate relation
```

---

## 6.2 不要求人为规定最终 Winner

如果：

```text
Y
```

最后成功：

```text
A -> Y
```

则允许。

如果：

```text
Z
```

最后成功：

```text
A -> Z
```

也允许。

本阶段不定义：

```text
Y 必须赢
Z 必须赢
```

这种业务排序。

唯一要求：

> 最终数据库必须满足 Relation Invariant。

---

# 7. P0 — Relocate Transaction Isolation

确认 Relocate 的完整操作必须处于同一事务：

```text
BEGIN
 ↓
Validate Work
 ↓
Validate Old Asset
 ↓
Validate Old Relation
 ↓
Validate New Asset
 ↓
Upsert New Asset
 ↓
Delete Old ATTACHES
 ↓
Check Existing Target
 ↓
Insert New ATTACHES if necessary
 ↓
COMMIT
```

任何一步失败：

```text
ROLLBACK
```

---

## 7.1 特别验证

以下错误不得产生半完成状态：

### Case A

```text
Work 不存在
```

结果：

```text
旧 Relation 不变
新 Asset 不出现
```

### Case B

```text
Old Asset 不存在
```

结果：

```text
旧状态不变
```

### Case C

```text
Old Asset 不是 Directory
```

结果：

```text
旧状态不变
```

### Case D

```text
Old Relation 不是 ATTACHES
```

结果：

```text
旧状态不变
```

### Case E

```text
New Asset 非 Directory
```

结果：

```text
旧状态不变
```

### Case F

```text
新 Relation 写入失败
```

结果：

```text
Old ATTACHES 必须恢复
```

---

# 8. P1 — Relation Duplicate Invariant

目前 Core 已经提供重复 Relation 幂等保护。

v0.5 必须进一步确认：

> 直接从 Repository / Database 层执行写操作时，也不能轻易制造重复的同向同类型 Relation。

---

## 8.1 Relation Identity

定义 Relation Logical Identity：

```text
(source_id,
 relation_type,
 target_id)
```

以下视为同一逻辑 Relation：

```text
A -> sequel_of -> B
A -> sequel_of -> B
```

第二次必须：

```text
Idempotent Success
```

---

## 8.2 方向必须参与 Identity

以下不是同一个 Relation：

```text
A -> sequel_of -> B
B -> sequel_of -> A
```

同样：

```text
A -> sequel_of -> B
B -> prequel_of -> A
```

可以同时存在。

不得使用：

```text
unordered(source_id, target_id)
```

作为 Relation Identity。

---

# 9. P1 — Historical Corruption Detection

不要求 v0.5 自动修改历史数据。

但是增加：

```rust
LoomaCore::validate_work_integrity(...)
```

或者等价的只读 Integrity API。

---

## 9.1 Integrity 检查项目

检查：

### Work

```text
Work Entity 是否存在
```

### Directory Attachment

检查：

```text
Work -> Asset
relation_type = attaches
Asset.kind = directory
```

### Primary

检查：

```text
primary_count <= 1
```

### Relations

检查：

```text
source != target
source exists
target exists
```

---

## 9.2 返回结构

建议：

```rust
IntegrityReport {
    valid: bool,
    violations: Vec<IntegrityViolation>,
}
```

例如：

```text
IntegrityViolation::DuplicatePrimary
IntegrityViolation::InvalidAttachmentDirection
IntegrityViolation::InvalidAttachmentAssetKind
IntegrityViolation::SelfRelation
IntegrityViolation::MissingRelationEndpoint
IntegrityViolation::DuplicateRelation
```

---

# 10. P1 — Integrity CLI

增加：

```bash
looma work check <WORK_ID>
```

输出：

### 正常

```text
Work Integrity: PASS
No violations found.
```

### 异常

```text
Work Integrity: FAIL

- Duplicate ATTACHES relation
- Invalid Asset Kind
- 2 Primary External References
```

要求：

```text
exit code 0 = PASS
exit code != 0 = FAIL
```

禁止：

```text
发现错误但 exit code 0
```

---

# 11. P1 — Integrity MCP

新增统一 MCP 工具：

```text
check_work_integrity
```

输入：

```json
{
  "work_id": "..."
}
```

返回：

```json
{
  "valid": true,
  "violations": []
}
```

异常：

```json
{
  "valid": false,
  "violations": [
    {
      "kind": "duplicate_primary",
      "message": "..."
    }
  ]
}
```

MCP 不得：

```text
吞掉错误
```

也不得：

```text
伪造成功
```

---

# 12. P1 — Cross-Layer Read Consistency

增加一个完整聚合读取测试：

```text
Work
├── Identity
├── Metadata
├── Lifecycle
├── Progress
├── Aliases
├── Assets
├── ExternalReferences
├── Memories
├── Collections
└── Universe Relations
```

执行：

```text
create
→ update aliases
→ update status
→ update progress
→ add asset
→ add external reference
→ set primary
→ add memory
→ add universe relation
→ cold restart
→ read aggregate
```

必须确保：

```text
每一层数据均保持原值
```

并且：

```text
修改某一层
```

不会隐式修改其他层。

---

# 13. P2 — Audit Event Semantic Consistency

继续复用现有 Audit 机制。

对以下操作确认：

```text
work.create
work.update_aliases
work.update_status
work.update_progress
work.attach_directory
work.relocate_directory
work.set_primary_reference
work.create_relation
```

必须：

```text
成功操作 -> success audit
失败操作 -> failure audit
```

---

## 13.1 禁止伪成功 Audit

尤其是：

```text
Relocate failed
```

不能记录：

```text
work.relocate_directory = success
```

同理：

```text
set_primary failed
```

不能记录：

```text
success
```

---

# 14. P2 — Serialization Compatibility Regression

继续保持：

```text
Old Data -> New Binary = PASS
```

新增测试：

```text
test_v05_unknown_work_enum_forward_compatibility
```

模拟未知枚举：

```json
{
  "work_type": "future_media_type"
}
```

New Binary 必须能够安全加载。

映射：

```text
WorkType::Other
```

同理：

```text
future_status
```

映射：

```text
RecordStatus::Unknown
```

---

# 15. Tests AA - AO

本版本新增测试至少 15 个。

| Test | 测试名称 | 验证目标 |
|---|---|---|
| AA | `test_v05_work_directory_attachment_query_strictness` | 只返回合法 Work→Directory ATTACHES |
| AB | `test_v05_duplicate_relation_invariant` | 重复 Relation 不产生第二条 |
| AC | `test_v05_relation_direction_identity` | 方向参与 Relation Identity |
| AD | `test_v05_concurrent_primary_switching` | 并发切 Primary 后最多一个 |
| AE | `test_v05_concurrent_primary_creation` | 并发创建 Primary 后最多一个 |
| AF | `test_v05_concurrent_relocate_same_target` | 并发 Relocate 到同一目录不重复 |
| AG | `test_v05_concurrent_relocate_different_targets` | 并发 Relocate 到不同目录最终仍满足不变量 |
| AH | `test_v05_relocate_transaction_failure_preserves_old_state` | 中途失败旧状态完整保留 |
| AI | `test_v05_work_integrity_valid_state` | 正常 Work Integrity PASS |
| AJ | `test_v05_work_integrity_detects_duplicate_primary` | 能检测 Primary 异常 |
| AK | `test_v05_work_integrity_detects_invalid_attachment` | 能检测非法 Attachment |
| AL | `test_v05_work_integrity_detects_self_relation` | 能检测自关联 |
| AM | `test_v05_cli_integrity_exit_code` | CLI 正常/异常 exit code 正确 |
| AN | `test_v05_cross_layer_aggregate_consistency` | 全五层 + Connections 冷重启一致 |
| AO | `test_v05_unknown_enum_forward_compatibility` | 未知枚举安全降级 |

---

# 16. Full Regression Requirement

执行：

```bash
cargo test --workspace
```

要求：

```text
ALL PASS
0 failures
0 ignored
```

v0.4 基线：

```text
39 Workspace Tests
```

v0.5 新增：

```text
>= 15 tests
```

因此 Work Suite 至少：

```text
47 tests
```

Workspace 总数不得低于：

```text
54 tests
```

具体数量以实际测试套件为准，但：

> 不允许为了通过 v0.5 而删除、弱化或修改既有 v0.1–v0.4 测试。

---

# 17. Concurrency Test Requirements

并发测试必须是真实并发，而不是：

```text
for loop
```

伪装成并发。

至少使用：

```rust
std::thread
```

或：

```rust
tokio
```

等真实并发机制。

要求至少覆盖：

```text
8 threads
100 rounds
```

如果测试运行环境导致 SQLite contention：

允许合理 retry/backoff，但不得：

```text
catch error -> ignore
```

必须最终检查数据库不变量。

---

# 18. SQLite Concurrency Principle

不得为了通过测试简单粗暴地增加：

```text
sleep
```

或：

```text
retry until pass
```

来掩盖实际竞态。

优先确认：

```text
Connection Lock
Transaction Boundary
SQLite Transaction Semantics
Repository Atomicity
```

是否正确。

如果当前 Looma 架构使用：

```text
single shared SQLite connection + Mutex
```

不得为了 v0.5 强行重构成 connection pool。

本阶段目标不是数据库连接架构升级。

---

# 19. Forbidden Changes

本补丁明确禁止：

```text
新增 AnimeTable
新增 MangaTable
新增 BookTable
新增 MovieTable
新增 GameTable

新增 AnimeService
新增 MangaService
新增 BookService
新增 MovieService
新增 GameService

新增 AnimeProgress
新增 MangaProgress
新增 BookProgress

修改现有 SQLite Schema 以绕过 JSON 模型
```

除非发现现有数据库 Schema 存在无法通过当前架构表达的真实一致性问题，否则：

> v0.5 不进行 SQLite Schema Migration。

---

# 20. Compatibility

继续保持：

### Old Data -> New Binary

```text
PASS
```

必须能够读取：

```text
旧 aliases 缺失
旧 type_metadata 缺失
旧枚举字段缺失
```

---

### New Data -> Old Binary

```text
NOT GUARANTEED
```

不得宣称：

```text
Full Backward Compatibility
```

准确措辞继续使用：

```text
Forward-reading compatibility
```

---

# 21. Implementation Order

反重力必须按照以下顺序实施：

```text
Step 1
先实现 / 检查 Relation Integrity

↓

Step 2
实现 Primary Concurrent Safety

↓

Step 3
实现 Relocate Concurrent Safety

↓

Step 4
实现 Work Integrity Read-only Validator

↓

Step 5
接入 CLI / MCP

↓

Step 6
补齐 Audit Semantic Consistency

↓

Step 7
补齐 Compatibility Regression

↓

Step 8
运行完整测试

↓

Step 9
运行 Desktop Build

↓

Step 10
Git Commit + Push
```

---

# 22. Acceptance Criteria

最终验收必须至少满足：

```text
[P0] Relation Logical Identity 明确
[P0] Duplicate Relation 不产生重复
[P0] Primary count <= 1
[P0] Concurrent Primary Switching 安全
[P0] Concurrent Primary Creation 安全
[P0] Concurrent Relocate 安全
[P0] Relocate Failure Atomic
[P0] Work -> Directory ATTACHES 严格成立

[P1] Integrity Validator 可检测历史异常
[P1] CLI Integrity 可用
[P1] MCP Integrity 可用
[P1] Cross-layer aggregate persistence PASS

[P2] Audit success/failure 语义正确
[P2] Unknown Enum Forward-reading Compatibility PASS

[REGRESSION]
v0.1 PASS
v0.2 PASS
v0.3 PASS
v0.4 PASS
```

---

# 23. Final Delivery Report Requirements

实施完成后，必须输出一份完整：

```text
Looma Work Model Semantic Hardening v0.5
Implementation Audit & Acceptance Review Report
```

至少包含：

1. Modified Files
2. New APIs
3. Database Transaction Changes
4. Concurrency Strategy
5. Integrity Invariants
6. Tests AA–AO
7. `cargo test --workspace` 完整结果
8. Desktop Build 结果
9. Release Binary 大小
10. Git Commit Hash
11. Git Status
12. Compatibility Statement
13. Final Acceptance Matrix

最终必须明确：

```text
PASS / FAIL
```

不得仅报告：

```text
Implemented
```

---

# 24. Definition of Done

v0.5 不是“代码写完”即完成。

只有同时满足：

```text
Code implemented
+
Existing tests preserved
+
New tests pass
+
Concurrency tests pass
+
Integrity validator pass
+
CLI/MCP error semantics pass
+
Desktop build pass
+
Git working tree clean
```

才允许宣布：

```text
Looma Work Model Semantic Hardening v0.5
STATUS: ACCEPTED
```

---

# 25. Architecture End State

完成 v0.5 后，Looma Work 模型应达到：

```text
                         ┌─────────────────────────┐
                         │       Generic Work      │
                         └────────────┬────────────┘
                                      │
        ┌─────────────────────────────┼─────────────────────────────┐
        │                             │                             │
   Identity                      Metadata                      Lifecycle
        │                             │                             │
   id/title/aliases             type_metadata              status/progress
        │                             │                             │
        └─────────────────────────────┼─────────────────────────────┘
                                      │
                               Connections
                                      │
          ┌───────────────┬───────────┼───────────┬───────────────┐
          │               │           │           │               │
        Assets         External     Memory    Collection       Relations
                         Refs
                          │
                     Primary <= 1
                          │
                 ┌────────┴────────┐
                 │                 │
              Core             Database
                 │                 │
                 └───────┬─────────┘
                         │
                  Transactional
                    Invariants
                         │
              ┌──────────┼──────────┐
              │          │          │
          Relocate    Primary    Relation
          Atomicity   Uniqueness  Integrity
              │          │          │
              └──────────┼──────────┘
                         │
                  Integrity Validator
                         │
                  CLI / MCP / GUI
```

最终目标不是增加更多功能，而是让 Looma 的 Generic Work 模型具备：

```text
Semantic Correctness
+
Transactional Atomicity
+
Concurrency Safety
+
Historical Integrity Detection
+
Cross-layer Consistency
+
Protocol Consistency
```

并且继续保持：

```text
Zero Media-specific Tables
Zero Media-specific Services
Zero Work Identity Merging
Zero Hidden State Coupling
```

**这才是 v0.5 的完成标准。**