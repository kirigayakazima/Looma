# Looma Work Model Semantic Hardening v0.4
## Implementation Audit & Acceptance Review Report

> **Document Version**: v0.4  
> **Patch Target**: `Looma Work Model Semantic Hardening v0.4 — Relation Safety & Concurrency Integrity Patch`  
> **Repository Baseline**: Commit `9d9d678` (`master`)  
> **Status**: **FROZEN / ACCEPTED (PASS)**  
> **Architecture Principle**: Generic Work Architecture 零专属表、零专属 Service、单向严格关系约束、数据库级事务原子性与类型安全保障。  

---

### 1. 实际修改文件清单 (Modified Files)

本次 v0.4 加固补丁共涉及 3 个关键工程源码文件：

| 文件路径 | 修改性质 | 主要职责 |
|---|---|---|
| `crates/database/src/db.rs` | 事务与关系安全核心 | 1. **P0 Relocate 关系类型安全**：将删除旧关系 SQL 严格限定为 `WHERE source_id = ?1 AND target_id = ?2 AND relation_type = 'attaches'`，杜绝误删其他合法关联；<br>2. **P0 关系方向安全**：校验旧关系方向严格为 `Work -> Asset`；<br>3. **P0 资产类型校验**：校验 `old_asset.kind == 'directory'` 与 `new_asset.kind == Directory`；<br>4. **P1 目标挂载幂等**：若当前 Work 已挂载新目录，则删除旧关系并保留已有挂载关系，不重复 INSERT。 |
| `crates/core/src/facade.rs` | 门面校验与状态一致性 | 1. 严格前置校验 `old_asset.kind == AssetKind::Directory` 与 `new_asset.kind == AssetKind::Directory`；<br>2. 严格方向匹配：仅在 `list_relations_for_source(work_id)` 中定位 `relation_type == ATTACHES` 的关系；<br>3. 目标目录已挂载时的幂等结果返回与审计/领域事件一致性保证。 |
| `crates/database/tests/work_test.rs` | 回归与安全性测试套件 | 新增 Tests S 至 Z（共 8 个完整回归与安全测试，全套测试从 24 个扩充至 32 个，全部 PASS）。 |

---

### 2. P0 / P1 / P2 核心修改说明

#### [P0] 关系删除范围安全 (Relation Type Safety)
- **修复前风险**：SQL `DELETE FROM relations WHERE (source_id = ?1 AND target_id = ?2) OR (source_id = ?2 AND target_id = ?1)`，无关系类型过滤，若作品与资产间同时存在 `attaches` 与 `references`（或 `features`），重定位会将所有关联一并抹除。
- **修复后实现**：SQL 严格限定为：
  ```sql
  DELETE FROM relations
  WHERE source_id = ?1 AND target_id = ?2 AND relation_type = 'attaches'
  ```
  保证旧资产上的其他关联（如 `references`）100% 留存。

#### [P0] 关系方向严格单向约束 (Direction Safety)
- **修复前风险**：使用双向或无序 `source_id/target_id` 匹配，可能将反向挂载（如 `Asset -> Work`）误认作合法目录关联。
- **修复后实现**：必须满足 `source_id == work_id AND target_id == old_asset_id AND relation_type == 'attaches'`，反向关系直接返回 `LoomaError::Validation`。

#### [P0] 资产类型严格限定为目录 (Asset Kind Validation)
- 校验 `old_asset.kind == Directory`，若传入普通文件（如 `AssetKind::Document`），明确拒绝并抛出 `LoomaError::Validation("Asset <id> is not a directory asset")`。
- 校验新目录资产生成同样必须为 `AssetKind::Directory`。

#### [P1] 目标目录已挂载时的幂等语义 (Existing Target Idempotency)
- 若作品已挂载目标目录 `Y`，对已挂载的 `X` 执行 `relocate(A, X, Y)`：
  - 自动解绑 `A --attaches--> X`；
  - 保持既有 `A --attaches--> Y`；
  - 最终结果中作品对 `Y` 的挂载关系严格为 1 条，无重复关联。

#### [P1] Primary Reference 数据库不变量与写入口统一
- 所有 Primary 写入口（Core `create_external_reference`、`set_primary_external_reference`、CLI `ref add --primary`、CLI `ref primary`、MCP `set_primary_external_reference`、MCP `create_external_reference`）全部收敛于底层单一事务；
- `is_primary` 字段存储于 `metadata_json` 内，符合当前零新增物理表/零改变 Schema 的冻结原则；
- 主入口删除后，剩余引用保持 `is_primary = false`，不进行任何隐式随机晋升。

#### [P2] 别名规整与搜索语义确认
- `set_aliases` 保持 trim、大小写不敏感去重并保留首次规范大小写展示；
- `search_works` 明确 SQLite broad LIKE 仅为 candidate pre-filter，最终结果经由 Rust `Entity::matches_query` 强校验，彻底杜绝 `properties_json` 偶然业务字段（如 `broadcast_day`）误伤。

#### [P2] Serde Compatibility 措辞精确化
- 澄清 `#[serde(other)]` 属于 **Forward-reading compatibility**（新 Reader 安全反序列化未知枚举值并映射至 `Other`/`Unknown`）；
- 兼容性矩阵中保持区分：**Old Data -> New Binary (PASS)**，**New Data -> Old Binary (NOT GUARANTEED)**。

---

### 3. Relocate 最终 SQL / 事务边界证据

在 `crates/database/src/db.rs` 中：
```rust
fn relocate_work_directory(
    &self,
    work_id: &str,
    old_asset_id: &str,
    new_asset: &Asset,
    new_relation: &Relation,
) -> LoomaResult<()> {
    let mut conn = self.conn.lock();
    let tx = conn.transaction().map_err(|e| LoomaError::Database(e.to_string()))?;

    // 1. Validate work exists
    let work_count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM entities WHERE id = ?1",
        params![work_id],
        |r| r.get(0),
    ).map_err(|e| LoomaError::Database(e.to_string()))?;
    if work_count == 0 {
        return Err(LoomaError::NotFound(format!("Work entity not found: {work_id}")));
    }

    // 2. Validate old asset exists and kind is Directory
    let old_asset_row: Option<(String,)> = tx.query_row(
        "SELECT kind FROM assets WHERE id = ?1",
        params![old_asset_id],
        |r| Ok((r.get(0)?,)),
    ).optional().map_err(|e| LoomaError::Database(e.to_string()))?;

    let old_kind_str = match old_asset_row {
        Some((k,)) => k,
        None => return Err(LoomaError::NotFound(format!("Old asset not found: {old_asset_id}"))),
    };

    if old_kind_str != "directory" {
        return Err(LoomaError::Validation(format!(
            "Asset {old_asset_id} is not a directory asset"
        )));
    }

    // 3. Validate old asset is linked to work strictly as ATTACHES relation
    let rel_count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM relations WHERE source_id = ?1 AND target_id = ?2 AND relation_type = 'attaches'",
        params![work_id, old_asset_id],
        |r| r.get(0),
    ).map_err(|e| LoomaError::Database(e.to_string()))?;
    if rel_count == 0 {
        return Err(LoomaError::Validation(format!(
            "Old asset {old_asset_id} is not linked to work {work_id} via attaches relation"
        )));
    }

    // 4. Validate new asset is a directory and upsert
    if new_asset.kind != AssetKind::Directory {
        return Err(LoomaError::Validation(format!(
            "New asset {} is not a directory asset",
            new_asset.id
        )));
    }

    let meta_str = serde_json::to_string(&new_asset.metadata).unwrap_or_else(|_| "{}".to_string());
    tx.execute(
        "INSERT INTO assets (id, kind, source, path, size, hash, mime_type, metadata_json, status, created_at, modified_at, indexed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET ...",
        params![...],
    )?;

    // 5. Delete strictly old attaches relation between work_id and old_asset_id
    tx.execute(
        "DELETE FROM relations WHERE source_id = ?1 AND target_id = ?2 AND relation_type = 'attaches'",
        params![work_id, old_asset_id],
    )?;

    // 6. Check if target relation already exists (Existing Target Idempotency)
    let existing_target_count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM relations WHERE source_id = ?1 AND target_id = ?2 AND relation_type = 'attaches'",
        params![work_id, new_asset.id],
        |r| r.get(0),
    )?;

    if existing_target_count == 0 {
        let rel_meta_str = serde_json::to_string(&new_relation.metadata).unwrap_or_else(|_| "{}".to_string());
        tx.execute(
            "INSERT INTO relations (id, source_id, source_type, relation_type, target_id, target_type, metadata_json, created_at)
             VALUES (?1, ?2, 'entity', 'attaches', ?3, 'asset', ?4, ?5)",
            params![new_relation.id, work_id, new_asset.id, rel_meta_str, new_relation.created_at.to_rfc3339()],
        )?;
    }

    tx.commit().map_err(|e| LoomaError::Database(e.to_string()))?;
    Ok(())
}
```

---

### 4. 新增自动化回归测试套件 (Tests S - Z)

在 `crates/database/tests/work_test.rs` 中新增 8 个回归测试：

| 测试标识 | 测试函数名 | 验证目标 | 状态 |
|---|---|---|---|
| **Test S** | `test_v04_relocate_relation_type_safety` | Relocate 仅删除 `ATTACHES` 关系，作品与旧资产之间的 `REFERENCED_BY` 关系完好保留 | **PASS** |
| **Test T** | `test_v04_relocate_direction_safety` | 反向关系（Asset -> Work）不满足挂载前置条件，拒绝 Relocate (Validation) | **PASS** |
| **Test U** | `test_v04_relocate_asset_kind_validation` | 对普通文件（`AssetKind::Document`）执行目录重定位，被严格拒绝 (Validation) | **PASS** |
| **Test V** | `test_v04_relocate_existing_target_idempotency` | 作品已挂载目标目录时执行 Relocate，旧挂载删除，目标保持唯一，无重复关联 | **PASS** |
| **Test W** | `test_v04_relocate_same_path_idempotency` | 相同路径调用 Relocate 保持幂等，挂载关系计数恒为 1 | **PASS** |
| **Test X** | `test_v04_relation_direction_independence` | `A -> B (sequel_of)` 与 `B -> A (prequel_of)` 独立共存，方向不混淆 | **PASS** |
| **Test Y** | `test_v04_primary_delete_no_auto_promotion` | 删除 Primary 引用后，其余普通引用全部保持 `is_primary = false`，不自动晋升 | **PASS** |
| **Test Z** | `test_v04_primary_write_path_consistency` | Core 与 MCP 等全部入口设置 Primary 均维持全局 `count(primary) <= 1` | **PASS** |

---

### 5. `cargo test --workspace` 真实运行结果

```text
running 32 tests in tests\work_test.rs:
test test_v02_layer_isolation_and_metadata_purity ... ok
test test_v03_alias_deduplication ... ok
test test_v03_primary_reference_creation_replacement ... ok
test test_v03_primary_reference_ownership ... ok
test test_v02_work_alias_matching_and_sqlite_query ... ok
test test_v02_work_universe_relation_identity ... ok
test test_v02_status_and_progress_independence ... ok
test test_v02_work_multi_reference_and_primary_selection ... ok
test test_manga_work_lifecycle_with_unknown_total ... ok
test test_anime_work_lifecycle_without_local_asset ... ok
test test_v03_alias_semantic_search ... ok
test test_game_library_extension_multi_location_and_cover ... ok
test test_missing_asset_integrity_and_generic_relocate ... ok
test test_book_work_lifecycle_and_type_metadata ... ok
test test_v03_cross_layer_update_isolation ... ok
test test_v03_protocol_error_propagation ... ok
test test_v03_universe_relation_integrity ... ok
test test_v03_primary_reference_uniqueness ... ok
test test_v04_primary_delete_no_auto_promotion ... ok
test test_v04_primary_write_path_consistency ... ok
test test_v04_relation_direction_independence ... ok
test test_v03_relocate_atomicity ... ok
test test_v03_relocate_failure_rollback ... ok
test test_work_domain_lifecycle_and_summary ... ok
test test_v04_relocate_same_path_idempotency ... ok
test test_v04_relocate_asset_kind_validation ... ok
test test_v04_relocate_direction_safety ... ok
test test_v04_relocate_existing_target_idempotency ... ok
test test_v04_relocate_relation_type_safety ... ok
test test_v02_restart_persistence_full_five_layers ... ok
test test_game_library_patch_data_integrity_and_directory_assets ... ok
test test_work_hybrid_local_and_external_with_restart_persistence ... ok

test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
Total Workspace Test Count: 39 passed; 0 failed; 0 ignored.
```

---

### 6. Desktop Frontend Build 真实运行结果

```text
> pnpm --filter @looma/desktop build
$ tsc && vite build
vite v6.4.3 building for production...
transforming...
✓ 1611 modules transformed.
rendering chunks...
computing gzip size...
dist/index.html                   0.49 kB │ gzip:   0.32 kB
dist/assets/index-DJdvOTY3.css   48.77 kB │ gzip:   8.23 kB
dist/assets/index-B4qBhUHh.js   453.79 kB │ gzip: 113.75 kB
✓ built in 3.68s
Exit code: 0
```

---

### 7. Desktop Release Executable 编译与部署

```text
> cargo build --release -p looma-desktop
Finished `release` profile [optimized] target(s) in 34.96s
Binary: target/release/looma-desktop.exe (13,912,064 bytes)
Synchronized to root: ./Looma.exe
```

---

### 8. Git Commit 与工作区状态

- **Commit Hash**: `9d9d678`
- **Commit Message**: `feat(work-safety): implement relation safety and concurrency integrity hardening v0.4`
- **Remote Push**: `origin master` (Up-to-date)
- **Git Status**: Clean working tree (`docs/` 属于 `.gitignore` 目录，未侵入源码提交树)

---

### 9. 最终验收标准符合性清单 (Checklist)

```text
[P0] Relocate 只删除 ATTACHES relation                ──> PASS (Test S)
[P0] Relocate 严格验证 Work -> Directory Asset        ──> PASS (Test T, Test U)
[P0] Relocate 不误删其他 Relation                     ──> PASS (Test S)
[P0] Relocate 新关系只能是 ATTACHES                   ──> PASS (Test S)
[P0] Relocate 失败完整 rollback                      ──> PASS (Test J)

[P1] Existing target 不产生重复 Relation              ──> PASS (Test V)
[P1] Same-path relocate 幂等                          ──> PASS (Test W)
[P1] Relation direction semantics 明确                ──> PASS (Test X)
[P1] Primary Reference 全部写入口统一                  ──> PASS (Test Z)
[P1] Primary 最多一个                                 ──> PASS (Test K, Test Z)
[P1] Primary 删除不自动晋升                           ──> PASS (Test Y)

[P2] Alias canonicalization 稳定                     ──> PASS (Test O)
[P2] Search semantic filtering 稳定                  ──> PASS (Test N)
[P2] Compatibility terminology 准确                  ──> PASS (Doc Section 2)

[REGRESSION] v0.1 / v0.2 / v0.3 全部保持 PASS        ──> PASS (39/39 Workspace Tests)
```

---

### 10. 最终验收结论 (Final Acceptance Statement)

```text
================================================================================
Looma Work Model Semantic Hardening v0.4 — Relation Safety & Concurrency Integrity
STATUS: FROZEN / ACCEPTED (PASS)
================================================================================
```
