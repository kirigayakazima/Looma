# Looma Work Model Semantic Hardening v0.5
## Implementation Audit & Acceptance Review Report

> **Document Version**: v0.5  
> **Patch Target**: `Looma Work Model Semantic Hardening v0.5 — Invariant & Concurrency Integrity Patch`  
> **Repository Baseline**: Commit `9d9d678` -> Next Release  
> **Status**: **FROZEN / ACCEPTED (PASS)**  
> **Architecture Principle**: Generic Work Architecture 零媒体专属表、零媒体专属 Service、零硬编码耦合、严格数据库事务原子性、严格只读完整性校验与并发安全保障。

---

### 1. 实际修改文件清单 (Modified Files)

本次 v0.5 加固补丁共涉及 6 个工程源码文件：

| 文件路径 | 修改性质 | 主要职责 |
|---|---|---|
| `crates/core/src/models/work.rs` | 模型层 | 1. 新增 `IntegrityViolation` 与 `IntegrityReport` 结构；<br>2. 保持 `WorkType` 与 `RecordStatus` 的 `#[serde(other)]` 前向读取兼容。 |
| `crates/core/src/facade.rs` | 核心门面与数据一致性 | 1. 派生 `#[derive(Clone)]` 支持多线程无缝共享；<br>2. 实现严格查询 `list_work_directory_attachments`；<br>3. 实现只读不变量校验 `validate_work_integrity`；<br>4. 统一 `relocate_work_directory`、`set_primary_external_reference`、`create_relation` 审计失败记录机制，杜绝伪成功。 |
| `crates/database/src/db.rs` | 数据库仓储层 | 在 `create_relation` 底层直接增加 `(source_id, relation_type, target_id)` 唯一性检查，直接保证 Repository 层调用时的幂等性与 Invariant A/D。 |
| `crates/cli/src/main.rs` | 命令行入口 | 新增 `looma work check <WORK_ID>`，校验不变量并在发现异常时返回非零 exit code。 |
| `crates/mcp/src/server.rs` | MCP 协议服务 | 新增 `check_work_integrity` 工具注册与执行，提供只读完整性校验结果。 |
| `crates/database/tests/work_test.rs` | 回归与安全性测试套件 | 新增 Tests AA 至 AO（共 15 个完整并发与不变量测试用例，work_test 从 32 个扩充至 47 个，全部 PASS）。 |

---

### 2. 新增公共 API (New APIs)

#### 2.1 Core API
```rust
// 1. 严格工作目录挂载查询
pub fn list_work_directory_attachments(
    &self,
    work_id: &str,
) -> LoomaResult<Vec<(Asset, Relation)>>;

// 2. 只读完整性检查
pub fn validate_work_integrity(
    &self,
    work_id: &str,
) -> LoomaResult<IntegrityReport>;
```

#### 2.2 CLI 命令
```bash
looma work check <WORK_ID>
```
- **PASS 输出**：
  ```text
  Work Integrity: PASS
  No violations found.
  ```
  `exit code = 0`
- **FAIL 输出**：
  ```text
  Work Integrity: FAIL

  - <violation message 1>
  - <violation message 2>
  ```
  `exit code = 1`

#### 2.3 MCP 工具
- **Tool Name**: `check_work_integrity`
- **Permission Level**: `ReadOnly`
- **Input**:
  ```json
  { "work_id": "work_12345678" }
  ```
- **Output**:
  ```json
  {
    "valid": true,
    "violations": []
  }
  ```

---

### 3. 数据库事务与仓储修改 (Database Transaction Changes)

1. **Relation Logical Identity 强约束**：
   - 关系逻辑唯一性定义为三元组 `(source_id, relation_type, target_id)`；
   - 在 `LoomaDb::create_relation` 中增加底层检查：
     ```sql
     SELECT COUNT(*) FROM relations
     WHERE source_id = ?1 AND relation_type = ?2 AND target_id = ?3
     ```
   - 若已存在同向同类型关系，直接返回 `Ok(())`（幂等），防止产生重复关联。
2. **Relocate 完整事务隔离与原子回滚**：
   - 单一数据库事务内完成：验证实体存在性 -> 验证旧资产存在与 Directory 类型 -> 验证旧 attaches 关系 -> 写入新资产 -> 删除旧 attaches -> 校验目标是否已挂载 -> 必要时插入新 attaches；
   - 任何一步失败，事务完全回滚，旧目录挂载 100% 保持不变，同时门面记录失败审计日志。

---

### 4. 并发保证策略 (Concurrency Strategy)

1. **SQLite 共享连接锁机制**：
   - 当前架构采用 `Arc<Mutex<Connection>>`，天然保证进程内所有多线程并发事务的串行化与序列一致性；
   - `LoomaCore` 派生 `Clone`，多线程并发调用共享底层 `Arc<dyn VaultRepository>`。
2. **并发测试覆盖**：
   - `test_v05_concurrent_primary_switching`：8 线程并发切换同一作品的 Primary 引用，校验最终 `count(primary) == 1`；
   - `test_v05_concurrent_primary_creation`：8 线程并发创建带有 `is_primary = true` 的引用，校验最终 `count(primary) == 1`；
   - `test_v05_concurrent_relocate_same_target`：8 线程同时将作品重定位到同一目录，校验最终挂载关系严格为 1 条且无重复；
   - `test_v05_concurrent_relocate_different_targets`：8 线程竞争重定位到不同目录，校验旧目录完全脱钩，最终挂载关系严格为 1 条，无悬空或冲突状态。

---

### 5. 六大不变量落实情况 (Integrity Invariants)

- **Invariant A (Attachment Uniqueness)**：1 个作品对 1 个目标目录在任何时刻最多只有 1 条 `attaches` 关系。
- **Invariant B (Primary Reference Uniqueness)**：1 个 Entity 在任何时刻最多只有 1 个 `is_primary = true` 的外部引用；删除主引用不进行隐式自动晋升。
- **Invariant C (Strict Attachment Target & Kind)**：`attaches` 关系严格满足 `Work -> Directory Asset`，反向关联或非目录文件全部被拒绝。
- **Invariant D (Direction Independence)**：方向严格参与 Relation 身份识别，`A -> sequel_of -> B` 与 `B -> sequel_of -> A`、`B -> prequel_of -> A` 独立共存。
- **Invariant E (Relocate Atomic Rollback)**：任何阶段失败，旧状态完整留存，无半完成污点。
- **Invariant F (Post-Concurrency State)**：并发写竞争结束后，数据库最终状态必须满足上述所有不变量。

---

### 6. Tests AA–AO 完整清单与执行结果

| 编号 | 测试用例名称 | 验证核心 | 结果 |
|---|---|---|---|
| **AA** | `test_v05_work_directory_attachment_query_strictness` | 严格只返回合法 Work→Directory ATTACHES，排除反向、非目录及其他关系 | **PASS** |
| **AB** | `test_v05_duplicate_relation_invariant` | 重复 Relation 在底层 Repository 同样保持幂等，不产生第二条数据 | **PASS** |
| **AC** | `test_v05_relation_direction_identity` | 关系方向独立参与身份判定，反向及对称逆关系互不干扰 | **PASS** |
| **AD** | `test_v05_concurrent_primary_switching` | 8 线程并发切换主引用，终态保证 `count(primary) == 1` | **PASS** |
| **AE** | `test_v05_concurrent_primary_creation` | 8 线程并发创建主引用，终态保证 `count(primary) == 1` | **PASS** |
| **AF** | `test_v05_concurrent_relocate_same_target` | 8 线程并发重定位到同一目录，无重复关系产生 | **PASS** |
| **AG** | `test_v05_concurrent_relocate_different_targets` | 8 线程竞争重定位到不同目录，旧目录完全解绑且最终只有 1 个有效挂载 | **PASS** |
| **AH** | `test_v05_relocate_transaction_failure_preserves_old_state` | 目标路径非法或中途失败时事务完全回滚，旧挂载完整留存并记录失败审计 | **PASS** |
| **AI** | `test_v05_work_integrity_valid_state` | 正常合法 Work 实体通过 `validate_work_integrity` 校验 | **PASS** |
| **AJ** | `test_v05_work_integrity_detects_duplicate_primary` | 准确侦测历史损坏或外部注入的多 Primary 异常 (`duplicate_primary`) | **PASS** |
| **AK** | `test_v05_work_integrity_detects_invalid_attachment` | 准确侦测非目录挂载与反向挂载异常 | **PASS** |
| **AL** | `test_v05_work_integrity_detects_self_relation` | 准确侦测源与目标相同的自关联异常 (`self_relation`) | **PASS** |
| **AM** | `test_v05_cli_integrity_exit_code` | CLI 校验合法与非法实体时返回正确的退出代码 (0 vs 1) | **PASS** |
| **AN** | `test_v05_cross_layer_aggregate_consistency` | 全五层模型 + Connections 经历 Cold Restart 保持 100% 数据一致性 | **PASS** |
| **AO** | `test_v05_unknown_enum_forward_compatibility` | 未知 WorkType / RecordStatus 及未来属性前向安全降级读取 | **PASS** |

---

### 7. 全量测试与构建输出证据

#### 7.1 `cargo test --workspace` 结果
```text
running 0 tests (looma main)
running 3 tests (looma_core)
test models::work::tests::test_backward_compatibility_without_type_metadata ... ok
test models::work::tests::test_manga_and_book_type_metadata_overlays ... ok
test models::work::tests::test_work_metadata_serde_with_type_metadata ... ok

running 1 test (looma_database db_test)
test test_in_memory_db_and_services ... ok

running 47 tests (looma_database work_test)
test test_v02_layer_isolation_and_metadata_purity ... ok
test test_v03_alias_semantic_search ... ok
test test_v03_primary_reference_ownership ... ok
test test_book_work_lifecycle_and_type_metadata ... ok
test test_v03_primary_reference_creation_replacement ... ok
test test_v02_work_alias_matching_and_sqlite_query ... ok
test test_v02_work_multi_reference_and_primary_selection ... ok
test test_v02_status_and_progress_independence ... ok
test test_anime_work_lifecycle_without_local_asset ... ok
test test_missing_asset_integrity_and_generic_relocate ... ok
test test_game_library_extension_multi_location_and_cover ... ok
test test_v03_alias_deduplication ... ok
test test_v03_cross_layer_update_isolation ... ok
test test_manga_work_lifecycle_with_unknown_total ... ok
test test_v04_primary_delete_no_auto_promotion ... ok
test test_v03_universe_relation_integrity ... ok
test test_v03_protocol_error_propagation ... ok
test test_v02_work_universe_relation_identity ... ok
test test_v04_primary_write_path_consistency ... ok
test test_v03_primary_reference_uniqueness ... ok
test test_v04_relation_direction_independence ... ok
test test_v05_cli_integrity_exit_code ... ok
test test_v05_relation_direction_identity ... ok
test test_v05_duplicate_relation_invariant ... ok
test test_v05_unknown_enum_forward_compatibility ... ok
test test_v04_relocate_same_path_idempotency ... ok
test test_v04_relocate_direction_safety ... ok
test test_v04_relocate_asset_kind_validation ... ok
test test_v05_work_integrity_detects_duplicate_primary ... ok
test test_v03_relocate_atomicity ... ok
test test_v03_relocate_failure_rollback ... ok
test test_v04_relocate_relation_type_safety ... ok
test test_v05_work_integrity_detects_invalid_attachment ... ok
test test_v05_work_integrity_detects_self_relation ... ok
test test_v04_relocate_existing_target_idempotency ... ok
test test_work_domain_lifecycle_and_summary ... ok
test test_v02_restart_persistence_full_five_layers ... ok
test test_game_library_patch_data_integrity_and_directory_assets ... ok
test test_v05_relocate_transaction_failure_preserves_old_state ... ok
test test_v05_work_directory_attachment_query_strictness ... ok
test test_v05_concurrent_relocate_different_targets ... ok
test test_v05_concurrent_relocate_same_target ... ok
test test_v05_work_integrity_valid_state ... ok
test test_v05_cross_layer_aggregate_consistency ... ok
test test_work_hybrid_local_and_external_with_restart_persistence ... ok
test test_v05_concurrent_primary_creation ... ok
test test_v05_concurrent_primary_switching ... ok

running 3 tests (looma_scanner scanner_test)
test test_classifier ... ok
test test_hasher ... ok
test test_incremental_scanner ... ok

Total: 54 passed, 0 failed, 0 ignored. (100% PASS)
```

#### 7.2 前端与可执行文件打包结果
- **Desktop Frontend Build**：`pnpm --filter @looma/desktop build` -> 成功通过（3.78s）。
- **Desktop Release Build**：`cargo build --release -p looma-desktop` -> 成功编译（42.98s）。
- **Release Binary**：`./Looma.exe`（大小：**13,925,376 bytes**）。

---

### 8. 兼容性声明 (Compatibility Statement)

- **Old Data -> New Binary: PASS**  
  旧数据无论缺少 `aliases`、`type_metadata` 或为早期枚举格式，均由新版本二进制平滑读取。
- **New Data -> Old Binary: NOT GUARANTEED**  
  包含未来枚举值（如未知媒体类型）写入的数据，旧版本客户端无法识别，属于预期内的前向读取单向保证。

---

### 9. 最终验收矩阵 (Final Acceptance Matrix)

| 验收条目 | 规范级别 | 达成状态 |
|---|---|---|
| Relation Logical Identity 明确并参与存储唯一性 | P0 | **ACCEPTED** |
| Duplicate Relation 幂等保护覆盖 Repository 与 Core | P0 | **ACCEPTED** |
| Primary Reference 终态全局唯一 (`count <= 1`) | P0 | **ACCEPTED** |
| Concurrent Primary Switching 并发安全性 | P0 | **ACCEPTED** |
| Concurrent Primary Creation 并发安全性 | P0 | **ACCEPTED** |
| Concurrent Relocate (同目标/异目标) 并发安全性 | P0 | **ACCEPTED** |
| Relocate 失败事务级原子回滚与审计失败记录 | P0 | **ACCEPTED** |
| Work -> Directory ATTACHES 严格类型与方向校验 | P0 | **ACCEPTED** |
| 只读 Work 完整性校验器 `validate_work_integrity` | P1 | **ACCEPTED** |
| CLI `looma work check` 命令与错误退出码规范 | P1 | **ACCEPTED** |
| MCP `check_work_integrity` 只读协议工具 | P1 | **ACCEPTED** |
| Cross-Layer Aggregate 冷重启与分层独立修改一致性 | P1 | **ACCEPTED** |
| Audit 语义一致性（禁止伪成功，明确记录 failure） | P2 | **ACCEPTED** |
| 未知枚举与未来字段前向读取兼容性 | P2 | **ACCEPTED** |
| v0.1–v0.4 既有功能无任何回归退化 | REGRESSION | **ACCEPTED** |

---

### 10. 验收判定 (Final Verdict)

**Looma Work Model Semantic Hardening v0.5**：**STATUS: ACCEPTED (PASS)**
