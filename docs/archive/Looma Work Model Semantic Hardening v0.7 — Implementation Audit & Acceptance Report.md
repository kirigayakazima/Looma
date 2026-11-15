# Looma Work Model Semantic Hardening v0.7 — Implementation Audit & Acceptance Report

## 1. 执行摘要 (Executive Summary)

本报告针对 `Looma Work Model Semantic Hardening v0.7 — Deterministic Repair & Integrity Recovery Patch.md` 审查意见进行了全面实施、深度测试与工程级验收。

在严格维持 **Generic Work Architecture**（无任何媒体专用物理表与专用服务）、**Zero Breaking Schema Migrations**（完全冻结既有 SQLite Schema）以及 **单一 Facade 门面架构** 的前提下，v0.7 成功将 v0.6 的只读完整性检测与预览提升为**确定性、可审计、事务性、显式写安全的完整性恢复闭环（Deterministic Integrity Recovery Loop）**。

### 核心加固成果：
1. **结构化修复计划（`IntegrityRepairPlan`）与确定性签名（`plan_id`）**：
   - 修复操作由强类型枚举 `IntegrityRepairOperation` 严格表达，仅支持 `RemoveDuplicateRelation`、`RemoveDuplicatePrimary`、`RemoveInvalidAttachment`、`RemoveSelfRelation`。
   - `plan_id` 由 `work_id`、排序后的 `violation_code` 以及排序后的目标对象 ID（`relation_id` / `reference_id`）经 SHA-256 哈希确定性生成（格式：`plan_<sha256_prefix>`）。相同数据库状态保证生成完全一致的 `plan_id`。
2. **只读 Preview 与显式 Write Repair 严格解耦**：
   - `preview_work_integrity_repair` 保持 100% 只读，严禁任何隐式或静默写操作。
   - `repair_work_integrity(actor, work_id, plan_id)` 为唯一写入口，必须传入有效的 `plan_id` 显式确认执行。
3. **过期计划保护（Stale Plan Protection）**：
   - 执行 Repair 时自动重算当前最新计划并校验 `plan_id == current_plan.plan_id`。
   - 一旦数据库状态在预览与执行间发生任何修改（如新增/删除关联、并发修改等），立即终止写操作并返回 `LoomaError::Conflict`，杜绝脏写。
4. **单事务原子执行与全面回滚（Single Transaction & Full Rollback）**：
   - 修复操作在底层 SQLite 的单一原子事务内执行。
   - 每一个修复操作执行前均强校验前置条件（Precondition check：目标记录是否存在、状态是否匹配）。若任意操作失败，事务完整回滚（`tx.rollback()`），不留任何半完成或脏状态。
5. **确定性胜者仲裁（Deterministic Winner Selection）**：
   - **重复 Relation**：以最早 `created_at` 优先，若相同则以字典序最小 `id` 胜出，仅保留一个胜者，删除多余重复项。
   - **重复 Primary Reference**：以最早 `created_at` 优先，字典序最小 `id` 次之；**胜者保持 `is_primary = true`，败者仅降级为 `is_primary = false`，绝对不删除外部引用实体记录**。
6. **不可修复损坏安全隔离**：
   - 对于 `MissingRelationEndpoint`（关联端点实体不存在）等无法确定业务意图的损坏，严禁进行猜测性自动删除或修复。
   - 计划标记为 `repairable = false`，清空 operations，若调用 repair 则返回 `LoomaError::Validation`。
7. **修复后强制完整性复核（Post-Repair Integrity Validation）**：
   - 事务提交后，强制自动调用 `validate_work_integrity(work_id)`。
   - 必须满足 `report.valid == true`。若复核失败，严禁返回假成功，立即向调用方返回 `LoomaError::Integrity` 并在审计日志中标记 `repair_completed_but_integrity_failed`。
8. **完备审计语义（Audit Semantics）**：
   - 预览：`action = integrity_repair_preview`, `write = false`, `result = success`。
   - 成功修复：`action = integrity_repair`, `write = true`, `result = success`。
   - 过期计划：`action = integrity_repair`, `write = true`, `result = failure`, `reason = stale_plan`。
   - 事务回滚：`action = integrity_repair`, `write = true`, `result = failure`, `reason = transaction_rollback`。
9. **CLI & MCP 完整集成**：
   - CLI 新增 `looma work repair <WORK_ID> --plan <PLAN_ID>`，精确映射退出码（0: PASS/NO-OP, 1: FAILED, 2: CONFLICT）。
   - MCP 严格权限分级：`check_work_integrity` (ReadOnly)、`preview_work_integrity_repair` (ReadOnly)、`repair_work_integrity` (Write)。

---

## 2. 验收矩阵对照 (Acceptance Matrix)

| # | 规范要求 (Requirement) | 优先级 | 实现状态 | 验证测试 / 依据 |
|---|---|---|---|---|
| 1 | `IntegrityRepairPlan` 模型升级 | P0 | **PASS** | `crates/core/src/models/work.rs` |
| 2 | Deterministic `plan_id` (SHA-256) | P0 | **PASS** | Test BA: `test_v07_repair_plan_determinism` |
| 3 | ReadOnly Preview | P0 | **PASS** | Test BB: `test_v07_repair_preview_is_still_read_only` |
| 4 | Explicit Write Repair API | P0 | **PASS** | `LoomaCore::repair_work_integrity` |
| 5 | Plan Staleness Protection | P0 | **PASS** | Test BI: `test_v07_stale_repair_plan_rejected` |
| 6 | Single Transaction Repair | P0 | **PASS** | `LoomaDb::repair_work_integrity` 单一事务实现 |
| 7 | Full Rollback on Failure | P0 | **PASS** | Test BH: `test_v07_repair_transaction_rollback` |
| 8 | Deterministic Duplicate Relation Winner | P0 | **PASS** | Test BC: `test_v07_duplicate_relation_repair` |
| 9 | Deterministic Primary Winner (Demotion Only) | P0 | **PASS** | Test BD: `test_v07_duplicate_primary_repair` |
| 10 | Invalid Attachment Safe Repair | P0 | **PASS** | Test BE: `test_v07_invalid_attachment_repair` |
| 11 | Missing Endpoint Not Auto-Repaired | P0 | **PASS** | Test BG: `test_v07_missing_endpoint_not_auto_repaired` |
| 12 | Self Relation Safe Repair | P1 | **PASS** | Test BF: `test_v07_self_relation_repair` |
| 13 | Post-Repair Integrity Validation | P0 | **PASS** | Test BJ: `test_v07_repair_post_validation` |
| 14 | Repair Audit Semantics | P1 | **PASS** | Test BK: `test_v07_repair_write_audit_semantics` |
| 15 | CLI Repair Command (`--plan`) | P1 | **PASS** | `crates/cli/src/main.rs` WorkCommands::Repair |
| 16 | MCP Write Tool (`repair_work_integrity`) | P1 | **PASS** | `crates/mcp/src/server.rs` |
| 17 | Preview/Repair Permission Separation | P0 | **PASS** | MCP tools: ReadOnly vs MetadataWrite |
| 18 | Concurrent Stale Plan Protection | P0 | **PASS** | Test BL: `test_v07_concurrent_stale_plan` |
| 19 | Unrelated Relation Preservation | P0 | **PASS** | Test BM: `test_v07_repair_preserves_unrelated_relations` |
| 20 | External Reference Preservation (No deletion) | P0 | **PASS** | Test BN: `test_v07_repair_preserves_external_references` |
| 21 | Cold Restart Persistence | P1 | **PASS** | Test BO: `test_v07_repair_cold_restart_persistence` |
| 22 | Full Workspace Regression | P0 | **PASS** | `cargo test --workspace` (80/80 PASS) |
| 23 | Desktop Build (Vite + TypeScript) | P1 | **PASS** | `pnpm build` (4.08s, 0 errors) |
| 24 | Release Build (`Looma.exe`) | P1 | **PASS** | Tauri 嵌入构建，生成 14,024,704 字节静态可执行文件 |
| 25 | Zero Dedicated Media Tables | P0 | **PASS** | 物理表仍仅为 `entities`, `relations`, `assets`, `memories`, `collections`, `external_references`, `audit_logs` |
| 26 | Zero Dedicated Media Services | P0 | **PASS** | 无任何 `AnimeService`, `GameService` 等专用类 |
| 27 | Zero WorkIntegrityService | P0 | **PASS** | 完整性检测与修复直接内聚于单一 `LoomaCore` Facade 与 Repository |

---

## 3. 新增专项测试逐项报告 (Tests BA – BO)

本次加固在 `crates/database/tests/work_test.rs` 中新增了 15 个针对性测试（编号 BA 至 BO），全部通过：

| 测试编号 | 测试函数名 | 验证目标 | 运行结果 |
|---|---|---|---|
| **BA** | `test_v07_repair_plan_determinism` | 相同数据库状态在多次预览中计算出完全一致的 SHA-256 `plan_id`、`operations` 与 `violations`。 | **PASS** |
| **BB** | `test_v07_repair_preview_is_still_read_only` | 执行 `preview_work_integrity_repair` 前后比对数据库所有表的总行数与快照，绝对无任何写操作。 | **PASS** |
| **BC** | `test_v07_duplicate_relation_repair` | 针对 3 条重复 Relation，严格按照“最早 created_at 优先，字典序最小 id 胜出”保留唯一胜者，正确删除其余项。 | **PASS** |
| **BD** | `test_v07_duplicate_primary_repair` | 针对多个 Primary External Reference，胜者保留 primary，败者降级为 `is_primary = false`，不删除记录行。 | **PASS** |
| **BE** | `test_v07_invalid_attachment_repair` | 正确清理非法的 ATTACHES 关联，同时保证被关联的基础 Asset 资产本体完整不被误删。 | **PASS** |
| **BF** | `test_v07_self_relation_repair` | 正确检出并安全清理自指向关联（`source_id == target_id`），修复后完整性检查有效。 | **PASS** |
| **BG** | `test_v07_missing_endpoint_not_auto_repaired` | 关联目标不存在（Missing Endpoint）时，计划标记为不可自动修复且不生成 operations；调用修复明确返回错误并拒绝静默修复。 | **PASS** |
| **BH** | `test_v07_repair_transaction_rollback` | 单事务保障：当修复计划中包含非法或不存在的目标时，前序已执行的修改完整原子回滚，原数据库状态保持不变。 | **PASS** |
| **BI** | `test_v07_stale_repair_plan_rejected` | 在预览获得计划后变更数据库数据，再次使用旧 `plan_id` 执行修复立即返回 `LoomaError::Conflict`。 | **PASS** |
| **BJ** | `test_v07_repair_post_validation` | 修复完成后自动执行后验完整性复核，返回有效报告且 `valid == true`。 | **PASS** |
| **BK** | `test_v07_repair_write_audit_semantics` | 验证 Preview (`write: false`)、Stale Plan Conflict (`reason: "stale_plan"`)、Repair Success (`write: true`) 审计日志精确记录。 | **PASS** |
| **BL** | `test_v07_concurrent_stale_plan` | 多线程并发执行相同计划：第一个线程修复成功后，第二个线程因数据库状态改变命中过期计划防护被拒绝。 | **PASS** |
| **BM** | `test_v07_repair_preserves_unrelated_relations` | 修复重复关系时，同 Work 上的其它合法关联（如 CREATED_BY、PREQUEL_OF）不受任何干扰与误删。 | **PASS** |
| **BN** | `test_v07_repair_preserves_external_references` | 修复重复 Primary 时，外部引用总行数在修复前后完全恒定，无任何记录行被删除。 | **PASS** |
| **BO** | `test_v07_repair_cold_restart_persistence` | 修复落盘后彻底关闭并销毁连接，重新以冷启动方式打开 SQLite 数据库，校验完整性仍然保持 100% 通过。 | **PASS** |

---

## 4. 全量回归测试输出 (Full Workspace Regression Output)

执行命令：
```bash
cargo test --workspace
```

实际运行终端输出片段：
```text
running 3 tests
test models::work::tests::test_backward_compatibility_without_type_metadata ... ok
test models::work::tests::test_manga_and_book_type_metadata_overlays ... ok
test models::work::tests::test_work_metadata_serde_with_type_metadata ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 1 test
test test_in_memory_db_and_services ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

running 73 tests
test test_v02_layer_isolation_and_metadata_purity ... ok
test test_book_work_lifecycle_and_type_metadata ... ok
test test_v02_work_alias_matching_and_sqlite_query ... ok
test test_v02_status_and_progress_independence ... ok
test test_v03_primary_reference_creation_replacement ... ok
test test_v02_work_multi_reference_and_primary_selection ... ok
test test_v03_alias_semantic_search ... ok
test test_v03_primary_reference_ownership ... ok
test test_game_library_extension_multi_location_and_cover ... ok
test test_v03_alias_deduplication ... ok
test test_manga_work_lifecycle_with_unknown_total ... ok
test test_v02_work_universe_relation_identity ... ok
test test_missing_asset_integrity_and_generic_relocate ... ok
test test_anime_work_lifecycle_without_local_asset ... ok
test test_v03_cross_layer_update_isolation ... ok
test test_v03_primary_reference_uniqueness ... ok
test test_v03_protocol_error_propagation ... ok
test test_v04_primary_delete_no_auto_promotion ... ok
test test_v04_relation_direction_independence ... ok
test test_v03_universe_relation_integrity ... ok
test test_v04_primary_write_path_consistency ... ok
test test_v05_cli_integrity_exit_code ... ok
test test_v05_duplicate_relation_invariant ... ok
test test_v05_relation_direction_identity ... ok
test test_v05_unknown_enum_forward_compatibility ... ok
test test_v04_relocate_asset_kind_validation ... ok
test test_v05_work_integrity_detects_duplicate_primary ... ok
test test_v04_relocate_direction_safety ... ok
test test_v04_relocate_same_path_idempotency ... ok
test test_v05_work_integrity_detects_invalid_attachment ... ok
test test_v05_work_integrity_detects_self_relation ... ok
test test_v06_attachment_integrity_machine_codes ... ok
test test_v03_relocate_atomicity ... ok
test test_v06_duplicate_primary_detected_after_direct_repository_corruption ... ok
test test_v06_integrity_preview_is_read_only ... ok
test test_v03_relocate_failure_rollback ... ok
test test_v06_integrity_violation_machine_codes ... ok
test test_v04_relocate_existing_target_idempotency ... ok
test test_v04_relocate_relation_type_safety ... ok
test test_v02_restart_persistence_full_five_layers ... ok
test test_v06_primary_creation_transactional ... ok
test test_v06_primary_zero_or_one_invariant ... ok
test test_v06_repository_duplicate_relation_invariant ... ok
test test_v06_integrity_report_statistics ... ok
test test_v06_relocate_failure_preserves_all_layers ... ok
test test_game_library_patch_data_integrity_and_directory_assets ... ok
test test_v05_relocate_transaction_failure_preserves_old_state ... ok
test test_v05_concurrent_relocate_same_target ... ok
test test_v05_concurrent_relocate_different_targets ... ok
test test_v06_duplicate_relation_detected_after_direct_repository_corruption ... ok
test test_v05_work_directory_attachment_query_strictness ... ok
test test_v05_cross_layer_aggregate_consistency ... ok
test test_v05_work_integrity_valid_state ... ok
test test_v06_cross_layer_recovery_preview_consistency ... ok
test test_v07_duplicate_relation_repair ... ok
test test_v07_invalid_attachment_repair ... ok
test test_v07_duplicate_primary_repair ... ok
test test_v07_missing_endpoint_not_auto_repaired ... ok
test test_work_domain_lifecycle_and_summary ... ok
test test_v07_concurrent_stale_plan ... ok
test test_v07_repair_cold_restart_persistence ... ok
test test_v07_repair_plan_determinism ... ok
test test_v07_repair_post_validation ... ok
test test_v07_repair_preserves_external_references ... ok
test test_v07_repair_transaction_rollback ... ok
test test_v07_repair_preserves_unrelated_relations ... ok
test test_v05_concurrent_primary_creation ... ok
test test_v07_repair_preview_is_still_read_only ... ok
test test_v07_self_relation_repair ... ok
test test_v07_repair_write_audit_semantics ... ok
test test_v07_stale_repair_plan_rejected ... ok
test test_work_hybrid_local_and_external_with_restart_persistence ... ok
test test_v05_concurrent_primary_switching ... ok
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s

running 3 tests
test test_classifier ... ok
test test_hasher ... ok
test test_incremental_scanner ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.11s

全量测试结果：80 passed; 0 failed; 0 ignored (100% 通过)
```

---

## 5. 前端与可执行文件打包验证 (Desktop Build Verification)

为防止桌面应用程序运行时出现“找不到本地 Web 服务/连接被拒”的开发模式残留问题，严格执行包含嵌入静态资源的 Tauri 构建流程：

1. **前端工程构建**：
   ```bash
   pnpm --filter @looma/desktop tauri build --no-bundle
   ```
   - TypeScript 类型检查：PASS
   - Vite 静态构建：`✓ built in 4.08s`，生成 HTML/CSS/JS 静态资源包。
2. **Rust Release 构建**：
   - 使用 `tauri-build` 宏将 `dist/` 完整打包并嵌入可执行二进制文件内。
   - `Finished release profile [optimized] in 50.89s`。
   - 输出路径：`target/release/looma-desktop.exe`。
3. **根目录分发部署**：
   - 复制并生成 `./Looma.exe`（文件大小：14,024,704 字节）。
   - 用户双击或在没有启动 Vite Dev Server 的离线环境下均可直接渲染运行完整的桌面 UI。

---

## 6. 最终验收结论 (Final Acceptance Conclusion)

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
PASS (80 passed, 0 failed, 0 ignored)

Desktop Build:
PASS

Release Build:
PASS (./Looma.exe 14,024,704 bytes)

Architecture Boundary:
PASS

STATUS:
FROZEN / ACCEPTED
```
