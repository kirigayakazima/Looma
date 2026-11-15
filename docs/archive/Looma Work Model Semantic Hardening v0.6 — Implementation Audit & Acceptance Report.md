# Looma Work Model Semantic Hardening v0.6 — Implementation Audit & Acceptance Report

## 1. 执行摘要 (Executive Summary)

本报告针对 `Looma Work Model Semantic Hardening v0.6 — Database Invariant & Recovery Integrity Patch.md` 审查意见进行了完整实施与工程级验收。

在保持 **Zero Dedicated Media Tables**（零媒体专用物理表）、**Zero Dedicated Media Services**（零媒体专用服务）、**Zero Breaking Schema Migrations**（完全冻结既有 SQLite Schema）以及 **Work 五层解耦架构** 的前提下，v0.6 实现了以下关键加固：

1. **Relation 唯一性 Database Invariant**：严格确立 `(source_id, relation_type, target_id)` 参与方向判定的逻辑主键身份，由 Repository 单事务幂等操作保证最多存在一条。
2. **Primary External Reference 最终规范**：确立对于任意 Entity 始终满足 `count(is_primary = true) <= 1`（允许 0 条或 1 条），在单一事务内完成旧 Primary 降级与新 Primary 写入。
3. **稳定 Machine-Readable 错误码**：引入 `IntegrityViolationCode` 枚举（如 `duplicate_primary`, `duplicate_relation`, `invalid_attachment_kind`, `invalid_attachment_direction` 等），为 CLI、MCP、UI 提供跨平台稳定结构化语义，并附带 `entity_id`、`relation_id`、`reference_id` 定位上下文。
4. **IntegrityReport 检查统计信息**：扩展报告统计字段 `checked_relations`, `checked_references`, `checked_assets`，明确反馈检查深度。
5. **只读 Integrity Repair Preview**：新增 `preview_work_integrity_repair` API 与 MCP 工具，提供 100% 只读、确定性的修复计划预览（`IntegrityRepairPlan`），严格禁止静默写库与破坏性自动修复。
6. **明确并发语义边界**：在代码注释与架构规范中明确 `Arc<Mutex<Connection>>` 为进程内多线程串行化保证，明确指出多进程竞争与外部进程写入属于当前单进程保证范围之外（Out of Current Guarantee Boundary）。
7. **全量测试套件验收**：新增 Tests AP–AZ（11 项专项测试），工作区全量测试 65/65 全部通过（0 failure）。

---

## 2. 核心加固与设计决策 (Core Hardening & Design Decisions)

### 2.1 Relation 逻辑主键与方向参与判定

- **逻辑身份**：三元组 `(source_id, relation_type, target_id)`。
- **方向敏感性**：`A -> sequel_of -> B` 与 `B -> sequel_of -> A` 是两个完全独立合法的语义关系，严禁使用任何忽略方向的 canonicalization。
- **数据库保证**：在冻结物理 Schema 的约束下，由 `RelationRepository` 在事务中进行 `existence check + insert` 逻辑幂等处理，避免物理表内插入重复逻辑关系。历史可能存在的重复数据由 `validate_work_integrity` 精确检出。

### 2.2 Primary Reference 0 或 1 不变量

- **语义不变量**：`count(is_primary = true) <= 1`。Entity 可拥有 0 个 Primary，也可拥有 1 个 Primary，但严禁出现 >= 2 个 Primary。
- **事务单阶段更新**：创建或设为 Primary 时，在单一 SQLite 事务内原子执行 `demote existing -> insert/update new -> commit`。
- **Primary 删除与降级**：Primary 被删除或失效时不进行隐式自动晋升（`remaining[0] -> primary`），保持明确的用户选择权。

### 2.3 机器可读错误码体系 (IntegrityViolationCode)

新增枚举 `IntegrityViolationCode`：
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
    WorkNotFound,
    NotAWork,
    #[serde(other)]
    Unknown,
}
```

每个 `IntegrityViolation` 均包含：
- `code: IntegrityViolationCode` (用于程序逻辑、CLI 错误标识、MCP 输出)
- `kind: String` (向后兼容 v0.5)
- `message: String` (人类可读描述，不作为逻辑判断依据)
- `entity_id: Option<String>`, `relation_id: Option<String>`, `reference_id: Option<String>` (问题定位上下文)

### 2.4 修复预览只读设计 (Read-Only Repair Preview)

遵循 **“检测损坏 ≠ 擅自修复”** 原则：
- `preview_work_integrity_repair(work_id)` 生成只读计划 `IntegrityRepairPlan`：
  - `safe: bool`：对于缺失实体等不可自动决定的损坏标记为 `false`；对于重复关系或重复主引用的确定性降级标记为 `true`。
  - `actions: Vec<IntegrityRepairAction>`：包含 `kind` (`DemoteDuplicatePrimary`, `RemoveDuplicateRelation`, `DetachInvalidAttachment`, `RemoveSelfRelation` 等)、目标 ID 和详细参数。
- **只读保证**：预览期间无任何数据库写操作，前验与后验数据库状态及快照完全一致。

### 2.5 明确并发保证边界 (Concurrency Boundary Specification)

在 `crates/database/src/db.rs` 中的 `LoomaDb` 文档注释明确界定：
- **Guaranteed**：同一进程内、共享同一 `LoomaDb` 实例的多线程并发访问通过 `Arc<Mutex<Connection>>` 与事务边界串行化，保证完全的线程安全与事务隔离。
- **Out of Current Guarantee Boundary**：独立 OS 进程之间的 SQLite 文件竞争、外部 SQLite 写入进程、以及分布式/网络文件系统（NFS/SMB）并发访问，在当前架构下不在保证范围内，未来如有需要将引入跨进程锁服务。

---

## 3. CLI & MCP 工具实现对齐

### 3.1 CLI `looma work check <WORK_ID>`
- **验证通过**：
  ```text
  Work Integrity: PASS

  Checked:
    Relations: 4
    References: 3
    Assets: 2

  No violations found.
  ```
- **验证失败**：
  ```text
  Work Integrity: FAIL

  [duplicate_primary]
  Entity has more than one primary external reference. (reference_id: Some("..."))

  [invalid_attachment_kind]
  ATTACHES target is not a directory asset.

  Exit code: 1
  ```

### 3.2 MCP 工具接口
- `check_work_integrity`：权限维持 `ReadOnly`，输出包含 `valid`, `violations`（带 `code` 与上下文），以及 `checked_relations`, `checked_references`, `checked_assets`。
- `preview_work_integrity_repair`：注册为新 `ReadOnly` 工具，接受 `work_id` 参数，返回 `IntegrityRepairPlan`，无任何写副作用。

---

## 4. 验证与回归测试套件 (Verification & Acceptance Tests)

### 4.1 新增 v0.6 专项测试（Tests AP–AZ，共 11 项）

| 编号 | 测试用例名 | 验证目标 | 结果 |
| :--- | :--- | :--- | :--- |
| **AP** | `test_v06_repository_duplicate_relation_invariant` | Repository 层逻辑唯一性幂等；反向关系独立有效性 | **PASS** |
| **AQ** | `test_v06_integrity_violation_machine_codes` | 所有错误码稳定性、Serde 反序列化与未知枚举向前兼容性 | **PASS** |
| **AR** | `test_v06_primary_zero_or_one_invariant` | 0 primary 合法，1 primary 合法，2 primary 精确报错 | **PASS** |
| **AS** | `test_v06_primary_creation_transactional` | Primary 创建事务原子性；旧 Primary 自动降级且不产生孤立数据 | **PASS** |
| **AT** | `test_v06_relocate_failure_preserves_all_layers` | Relocate 失败回滚后，Identity、Metadata、Lifecycle、Progress、Connections 所有 5 层状态 100% 无损 | **PASS** |
| **AU** | `test_v06_attachment_integrity_machine_codes` | 反向 attachment、Document attachment、Missing endpoint 均返回稳定机器错误码 | **PASS** |
| **AV** | `test_v06_integrity_report_statistics` | `checked_relations`, `checked_references`, `checked_assets` 统计数值精准匹配 | **PASS** |
| **AW** | `test_v06_integrity_preview_is_read_only` | `preview_work_integrity_repair` 执行前后数据库状态无任何写变动 | **PASS** |
| **AX** | `test_v06_duplicate_primary_detected_after_direct_repository_corruption` | 绕过上层接口直接注入多个 Primary，完整检出 `duplicate_primary` | **PASS** |
| **AY** | `test_v06_duplicate_relation_detected_after_direct_repository_corruption` | 直接通过底层 SQLite 注入重复 Relation，完整检出 `duplicate_relation` | **PASS** |
| **AZ** | `test_v06_cross_layer_recovery_preview_consistency` | 跨层复杂损坏下，只读预览生成可解释且安全的修复步骤集合 | **PASS** |

### 4.2 全工作区测试汇总 (Workspace Test Summary)

```text
running 58 tests in tests\work_test.rs ... ok (58 passed; 0 failed)
running 1 test in tests\db_test.rs ... ok (1 passed; 0 failed)
running 3 tests in looma_core ... ok (3 passed; 0 failed)
running 3 tests in looma_scanner ... ok (3 passed; 0 failed)

Total Tests: 65 passed, 0 failed, 100% green.
```

---

## 5. 架构边界合规性核验 (Architectural Checklist)

- [x] **Zero Dedicated Media Tables**：没有任何 `anime_table`, `game_table` 等独立媒体表，完全沿用 `entities`, `assets`, `relations`, `memories`, `external_references` 统一基座。
- [x] **Zero Dedicated Media Services**：没有任何 `AnimeService`，统一通过 `LoomaCore` Facade 与基础 Repository 协同。
- [x] **No WorkIntegrityService**：完整度检查与修复预览由 `LoomaCore` Facade 直辖，严禁无谓的中间层膨胀。
- [x] **Zero Schema Breaking Changes**：未向 SQLite 引入不可兼容的物理约束，保持既有本地数据文件平滑兼容。
- [x] **Read-Only Repair Preview**：预览阶段严格只读，不进行任何自动写入修复。
- [x] **Explicit Concurrency Scope**：清晰说明多线程互斥（已保障）与跨进程并发（范围外）界限。
