# Looma Work Model Semantic Hardening v0.7

## Deterministic Repair & Integrity Recovery Patch

> **Patch Target**: `Looma Work Model Semantic Hardening v0.7`
>
> **Baseline**: v0.6 FROZEN / ACCEPTED
>
> **Goal**: 将 v0.6 的 ReadOnly Integrity Detection + Repair Preview 进一步推进为**确定性、可审计、事务原子、显式授权的 Integrity Recovery 闭环**。
>
> **Architecture Principle**:
>
> - Generic Work Architecture
> - Zero Dedicated Media Tables
> - Zero Dedicated Media Services
> - Zero WorkIntegrityService
> - Single LoomaCore Facade
> - Single SQLite transaction for each repair operation
> - ReadOnly preview 与 Write repair 严格分离
> - Repair 不允许隐式执行
> - Repair 必须可重复验证
> - Repair 必须具备 deterministic plan identity
> - Repair 失败必须完整 rollback

---

# 1. v0.7 核心目标

v0.6 已经具备：

1. `validate_work_integrity`
2. `IntegrityViolation`
3. `IntegrityReport`
4. `preview_work_integrity_repair`
5. 稳定 machine-readable violation codes
6. ReadOnly integrity preview
7. Repository duplicate relation invariant
8. Primary / Attachment / Relation integrity 检测
9. 跨层损坏场景测试

v0.7 **不再重复实现上述能力**。

本版本只解决一个问题：

> **当 `IntegrityReport` 发现可修复损坏时，如何生成一个确定性的 Repair Plan，并在用户明确调用 Write API 后，以单一事务安全执行，同时保证 Repair 前后 Integrity 状态可证明。**

---

# 2. 非目标 / 明确禁止

本版本禁止：

- 新增 `WorkIntegrityService`
- 新增 `AnimeService`
- 新增 `GameService`
- 新增 `MangaService`
- 新增任何媒体专属 SQLite 表
- 新增自动后台修复
- `validate_work_integrity()` 自动修改数据
- `preview_work_integrity_repair()` 修改数据
- CLI `work check` 自动修复
- MCP `check_work_integrity` 自动修复
- 启动时自动修复数据库
- 静默删除用户数据
- 静默删除 External Reference
- 静默删除 Relation
- 根据“猜测”自动选择 Primary Reference
- 根据路径扫描结果自动覆盖 Work Identity

任何 Repair 都必须由**显式 Write API**触发。

---

# 3. IntegrityRepairPlan 模型

## 3.1 新增模型

在：

```text
crates/core/src/models/work.rs
```

增加：

```rust
pub struct IntegrityRepairPlan {
    pub work_id: String,
    pub plan_id: String,
    pub generated_at: DateTime<Utc>,
    pub violations: Vec<IntegrityViolation>,
    pub operations: Vec<IntegrityRepairOperation>,
    pub repairable: bool,
}
```

增加：

```rust
pub enum IntegrityRepairOperation {
    RemoveDuplicateRelation {
        relation_id: String,
    },

    RemoveDuplicatePrimary {
        reference_id: String,
    },

    RemoveInvalidAttachment {
        relation_id: String,
    },

    RemoveSelfRelation {
        relation_id: String,
    },
}
```

---

# 4. Repair Operation 安全边界

## 4.1 Repair 只能针对明确识别的对象

每一个 Repair Operation 必须包含明确对象 ID。

禁止：

```text
DELETE FROM relations WHERE source_id = ?
```

这种模糊删除。

必须：

```text
DELETE FROM relations
WHERE id = ?
```

并且事务执行前再次验证：

- relation 当前仍然存在
- relation 当前状态仍然符合 Plan
- relation 对应的 violation 仍然成立

如果 Repair Plan 与数据库当前状态不一致：

```text
LoomaError::Conflict
```

Repair 必须失败。

---

# 5. Deterministic Plan Identity

## 5.1 Plan ID

`IntegrityRepairPlan.plan_id` 必须由：

```text
work_id
+
sorted violation codes
+
sorted target object ids
```

确定性生成。

相同数据库状态：

```text
same state
    ↓
same IntegrityReport
    ↓
same RepairPlan
    ↓
same plan_id
```

不得使用随机 UUID 作为唯一 Plan Identity。

时间字段：

```text
generated_at
```

可以变化，但不能影响 `plan_id`。

---

# 6. Repair Preview

已有：

```rust
preview_work_integrity_repair(
    work_id: &str
) -> LoomaResult<IntegrityRepairPlan>
```

v0.7 要求进一步保证：

### Case A

数据库没有 violation：

```text
repairable = false
operations = []
```

### Case B

存在可修复 violation：

```text
repairable = true
operations = [...]
```

### Case C

存在不可安全自动判断的 violation：

```text
repairable = false
operations = []
```

并保留原始 `violations`。

例如：

```text
missing_endpoint
```

不能自动猜测应该恢复哪个实体，因此不得自动生成 Repair Operation。

---

# 7. 新增显式 Repair API

## 7.1 Core

新增：

```rust
pub fn repair_work_integrity(
    &self,
    actor: &str,
    work_id: &str,
    plan_id: &str,
) -> LoomaResult<IntegrityReport>;
```

语义：

```text
preview
   ↓
user explicitly approves
   ↓
repair_work_integrity
   ↓
validate plan_id
   ↓
revalidate violations
   ↓
single SQLite transaction
   ↓
apply operations
   ↓
commit
   ↓
validate_work_integrity
   ↓
return final report
```

---

# 8. Repair 必须是单事务

`LoomaDb` 增加：

```rust
pub fn repair_work_integrity(
    &self,
    work_id: &str,
    plan: &IntegrityRepairPlan,
) -> LoomaResult<()>;
```

所有 operation 必须在：

```rust
let tx = conn.transaction()?;
```

内部完成。

禁止：

```text
operation 1 -> commit
operation 2 -> commit
operation 3 -> commit
```

必须：

```text
BEGIN
  operation 1
  operation 2
  operation 3
COMMIT
```

任何一个 operation 失败：

```text
ROLLBACK
```

---

# 9. Plan Staleness Protection

Repair API 不能只相信调用方传入的 `plan_id`。

执行时必须：

1. 重新生成当前 IntegrityRepairPlan
2. 比较：

```text
requested_plan_id == current_plan_id
```

3. 不一致：

```text
LoomaError::Conflict
```

错误信息必须明确：

```text
Integrity repair plan is stale.
Expected plan: ...
Current plan: ...
```

这样可以防止：

```text
Preview
   ↓
数据库发生变化
   ↓
旧 Plan 被执行
```

导致错误删除。

---

# 10. Repair Operation Preconditions

每一种 Repair Operation 都必须在执行前验证当前数据库状态。

## 10.1 Duplicate Relation

必须确认：

```text
relation_id exists
```

并且：

```text
same (source_id, relation_type, target_id)
```

仍存在重复关系。

删除规则必须 deterministic：

```text
保留最早 created_at 的 relation
```

如果 created_at 相同：

```text
保留 lexicographically smallest relation_id
```

其余重复关系才允许删除。

---

# 11. Duplicate Primary Repair

对于：

```text
duplicate_primary
```

禁止随机选择。

必须 deterministic：

```text
PRIMARY KEEP RULE
```

优先级：

1. `created_at` 最早
2. 若时间相同，`id` 字典序最小

其余：

```text
is_primary = false
```

注意：

> Repair 不能删除 External Reference，只允许解除 `is_primary` 状态。

---

# 12. Invalid Attachment Repair

对于：

```text
invalid_attachment
```

只有以下情况允许自动 Repair：

```text
Work -> Asset
relation_type = attaches
asset.kind != directory
```

或：

```text
Asset -> Work
relation_type = attaches
```

这种 relation 可以安全删除，因为其本身违反 Attachment Invariant。

但是：

```text
missing_endpoint
```

不得自动 Repair。

---

# 13. Self Relation Repair

对于：

```text
self_relation
```

如果：

```text
source_id == target_id
```

可以删除该 relation。

Repair 必须只删除：

```text
relation_id
```

不得影响同 Entity 的其他 Relation。

---

# 14. Repair 后强制完整性复核

Repair transaction commit 后：

```rust
let report = self.validate_work_integrity(work_id)?;
```

必须满足：

```text
report.valid == true
```

如果：

```text
report.valid == false
```

则 Repair 不得向调用者返回伪成功。

最佳实现：

```text
repair transaction
      ↓
commit
      ↓
validate
      ↓
FAIL
```

如果 commit 后发现仍有 violation：

必须返回：

```text
LoomaError::Integrity(...)
```

并在审计记录中标记：

```text
repair_completed_but_integrity_failed
```

注意：

> 不允许为了让最终状态变成 PASS 而继续进行第二轮隐式修复。

---

# 15. Audit Semantics

Repair API 必须区分：

### Preview

```text
action = integrity_repair_preview
success = true
write = false
```

### Repair Success

```text
action = integrity_repair
success = true
write = true
```

### Repair Conflict

```text
action = integrity_repair
success = false
reason = stale_plan
```

### Repair Failure

```text
action = integrity_repair
success = false
reason = transaction_rollback
```

禁止任何：

```text
failure -> fake success
```

---

# 16. CLI

新增：

```bash
looma work repair <WORK_ID> --plan <PLAN_ID>
```

### Success

```text
Work Integrity Repair: PASS

Plan: <PLAN_ID>

Applied operations: 3

Final Integrity: PASS
```

Exit code:

```text
0
```

### Stale Plan

```text
Work Integrity Repair: CONFLICT

Repair plan is stale.

Exit code:
2
```

### Repair Failure

```text
Work Integrity Repair: FAILED

Transaction rolled back.

Exit code:
1
```

### Nothing Repairable

```text
Work Integrity Repair: NO-OP

No repairable violations found.

Exit code:
0
```

---

# 17. MCP

新增：

```text
repair_work_integrity
```

权限：

```text
Write
```

Input：

```json
{
  "work_id": "work_123",
  "plan_id": "plan_..."
}
```

Output：

```json
{
  "success": true,
  "plan_id": "plan_...",
  "applied_operations": 3,
  "integrity": {
    "valid": true,
    "violations": []
  }
}
```

---

# 18. MCP Permission Boundary

必须明确：

```text
check_work_integrity
    ReadOnly

preview_work_integrity_repair
    ReadOnly

repair_work_integrity
    Write
```

三者不能混淆。

特别是：

```text
check_work_integrity
```

绝对不能产生数据库写入。

---

# 19. Repair Idempotency

重复执行同一个已经成功执行过的 Plan：

```text
repair_work_integrity(plan_id)
```

不能再次删除其他合法数据。

推荐行为：

```text
Conflict / Stale Plan
```

因为数据库状态已经发生变化。

禁止：

```text
same plan
→ silently execute again
→ accidentally delete unrelated relation
```

---

# 20. Concurrency

v0.7 必须继续保持 v0.5 的：

```text
Arc<Mutex<Connection>>
```

并发安全。

重点新增：

```text
Preview A
Preview B
    ↓
Repair A
    ↓
Repair B
```

其中 B 使用旧 Plan。

预期：

```text
Repair A -> PASS

Repair B -> CONFLICT / STALE_PLAN
```

不得让 B 使用旧快照继续执行。

---

# 21. New Tests AP+ / BA+ Range

新增至少以下测试。

| Test | Test Name | Requirement |
|---|---|---|
| BA | `test_v07_repair_plan_determinism` | 相同数据库状态产生相同 plan_id |
| BB | `test_v07_repair_preview_is_still_read_only` | Preview 前后数据库完全一致 |
| BC | `test_v07_duplicate_relation_repair` | 重复 Relation 只保留 deterministic winner |
| BD | `test_v07_duplicate_primary_repair` | 多 Primary 只保留 deterministic winner |
| BE | `test_v07_invalid_attachment_repair` | 非法 Attachment 可安全删除 |
| BF | `test_v07_self_relation_repair` | Self Relation 可安全删除 |
| BG | `test_v07_missing_endpoint_not_auto_repaired` | Missing endpoint 不生成危险自动修复 |
| BH | `test_v07_repair_transaction_rollback` | 任意 Repair operation 失败整体 rollback |
| BI | `test_v07_stale_repair_plan_rejected` | 数据变化后旧 Plan 被拒绝 |
| BJ | `test_v07_repair_post_validation` | Repair 完成后再次 Integrity Check |
| BK | `test_v07_repair_write_audit_semantics` | Preview/Repair/Conflict/Failure 审计语义准确 |
| BL | `test_v07_concurrent_stale_plan` | 并发 Repair 只有一个有效 Plan 可以成功 |
| BM | `test_v07_repair_preserves_unrelated_relations` | Repair 不影响无关 Relation |
| BN | `test_v07_repair_preserves_external_references` | Repair 不删除 External Reference |
| BO | `test_v07_repair_cold_restart_persistence` | Repair 后 Cold Restart 状态一致 |

---

# 22. Full Regression Requirements

必须执行：

```bash
cargo test --workspace
```

要求：

```text
0 failed
0 ignored
```

并且：

```text
v0.1
v0.2
v0.3
v0.4
v0.5
v0.6
```

全部 Regression PASS。

---

# 23. Frontend / Release Verification

继续执行：

```bash
pnpm --filter @looma/desktop build
```

以及：

```bash
cargo build --release -p looma-desktop
```

确认：

```text
TypeScript = PASS
Vite = PASS
Rust Release = PASS
Looma.exe = generated
```

---

# 24. Architecture Acceptance

v0.7 最终必须保持：

```text
                   ┌─────────────────────┐
                   │      LoomaCore      │
                   │       Facade        │
                   └──────────┬──────────┘
                              │
             ┌────────────────┼────────────────┐
             │                │                │
          Validate          Preview          Repair
          ReadOnly          ReadOnly           Write
             │                │                │
             └────────────────┼────────────────┘
                              │
                       VaultRepository
                              │
                         LoomaDb / SQLite
                              │
                     Single Transaction
```

不得引入：

```text
WorkIntegrityService
RepairService
MediaRepairService
AnimeRepairService
GameRepairService
```

---

# 25. Critical Safety Rules

以下规则属于 P0，不允许以“优化”为理由削弱：

### P0-1

Repair Preview 永远 ReadOnly。

### P0-2

Repair 必须显式调用。

### P0-3

Repair 必须验证 `plan_id`。

### P0-4

Repair 必须重新验证 violation。

### P0-5

Repair 必须单事务。

### P0-6

Repair 失败必须 rollback。

### P0-7

Repair 只能删除明确的 `relation_id` / 修改明确的 `reference_id`。

### P0-8

不得根据猜测修复 Missing Endpoint。

### P0-9

不得删除 External Reference 记录，只允许调整 `is_primary`。

### P0-10

Repair 完成后必须重新执行 Integrity Check。

---

# 26. Acceptance Matrix

| # | Requirement | Priority |
|---|---|---|
| 1 | IntegrityRepairPlan | P0 |
| 2 | Deterministic plan_id | P0 |
| 3 | ReadOnly Preview | P0 |
| 4 | Explicit Write Repair API | P0 |
| 5 | Plan Staleness Protection | P0 |
| 6 | Single Transaction Repair | P0 |
| 7 | Full Rollback | P0 |
| 8 | Deterministic Duplicate Relation Winner | P0 |
| 9 | Deterministic Primary Winner | P0 |
| 10 | Invalid Attachment Safe Repair | P0 |
| 11 | Missing Endpoint Not Auto-Repaired | P0 |
| 12 | Self Relation Safe Repair | P1 |
| 13 | Post-Repair Integrity Validation | P0 |
| 14 | Repair Audit Semantics | P1 |
| 15 | CLI Repair Command | P1 |
| 16 | MCP Write Tool | P1 |
| 17 | Preview/Repair Permission Separation | P0 |
| 18 | Concurrent Stale Plan Protection | P0 |
| 19 | Unrelated Relation Preservation | P0 |
| 20 | External Reference Preservation | P0 |
| 21 | Cold Restart Persistence | P1 |
| 22 | Full Workspace Regression | P0 |
| 23 | Desktop Build | P1 |
| 24 | Release Build | P1 |
| 25 | Zero Dedicated Media Tables | P0 |
| 26 | Zero Dedicated Media Services | P0 |
| 27 | Zero WorkIntegrityService | P0 |

---

# 27. Final Acceptance Criteria

只有同时满足以下条件才能标记：

```text
STATUS: FROZEN / ACCEPTED
```

必须：

```text
cargo test --workspace
    ↓
0 failed
```

并且：

```text
Preview = ReadOnly
Repair = Explicit Write
Repair = Transactional
Repair = Deterministic
Repair = Stale-plan protected
Repair = Post-validated
```

最终目标不是“Repair 能运行”。

而是：

> **任何 Integrity Repair 都必须能够回答：为什么修、修什么、为什么这个对象可以被修、修之前数据库是什么状态、修完后是否重新满足全部 Invariants，以及如果中途发生任何冲突，为什么没有留下半完成状态。**

---

# 28. Expected Final Report

Antigravity 完成后必须返回：

```text
Looma Work Model Semantic Hardening v0.7
— Deterministic Repair & Integrity Recovery

Implementation:
PASS

IntegrityRepairPlan:
PASS

Deterministic plan_id:
PASS

ReadOnly Preview:
PASS

Explicit Repair API:
PASS

Plan Staleness Protection:
PASS

Transactional Repair:
PASS

Rollback:
PASS

Deterministic Duplicate Resolution:
PASS

Post-Repair Validation:
PASS

CLI:
PASS

MCP:
PASS

Concurrency:
PASS

Regression:
PASS

cargo test --workspace:
PASS

Desktop Build:
PASS

Release Build:
PASS

Architecture Boundary:
PASS

STATUS:
FROZEN / ACCEPTED
```

**重要：不要只报告“代码已修改”。必须提供真实测试结果、真实 `cargo test --workspace` 输出、真实前端构建结果、真实 Release Build 结果，以及所有新增 Test 的逐项 PASS/FAIL 状态。**