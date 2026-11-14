# Looma v0.4 架构安全性与数据完整性强化报告 (Hardening Report)

**文档基准版本**：`208c165` / `8241b5e` / `615ba93` / `Final Hardening`  
**强化完成日期**：2026-11-14  
**目标定位**：针对 Looma v0.4 初版与二次 GPT 深度代码复核所发现的全部缺陷（包含统一事务还原、扫描生命周期静默失败、路径边界隔离与子目录墓碑继承、真实 MCP 进程级 stdio 会话、以及独立桌面端打包静态资源内嵌），执行系统性 Hardening，确保长期个人数字生活档案不可逆的安全与数据完整性。零新增业务功能、零破坏历史迁移、零半提交状态。

---

## 1. 核心修复与二次强化概览 (Summary of Hardening Changes)

| 问题分类 | 风险等级 | 修复前表现与漏洞根源 | 修复后状态与防护机制 | 覆盖测试 |
| :--- | :---: | :--- | :--- | :--- |
| **备份恢复原子性 (P0-A)** | **P0** | `restore_backup()` 逐条调用独立 Repository 写入，中途遇到错误（如外键或数据冲突）已写入数据仍留在目标数据库（半恢复脏状态）；缺少 Manifest 或 Checksum 时直接跳过校验；Checkpoint 与读取错误部分被 `let _` 或 `unwrap_or_default()` 吞掉。 | 采用单一 SQLite 统一事务 (`conn.transaction()`)，全表恢复（资产、实体、记忆、收藏、关系、外链、足迹、SourceRoot、生命周期、压制）统一在该事务内执行。若中途任何一条失败，事务立即完全回滚，目标数据库 100% 保持初态；强制要求 `manifest.json` 与 `vault_dump.json` 的 SHA-256 校验和以及 schema_version $\le 4$；所有 checkpoint 与数据库读取错误均严格传播 (`?`)。 | `backup_integrity_fault_test` (含故意注入脏数据回滚验证) |
| **扫描生命周期更新错误传播 (P0-B)** | **P0** | `LoomaCore::scan_directory()` 中生命周期关联使用 `let _ = self.repo.set_asset_lifecycle(&existing_lc)` 与 `set_asset_lifecycle(&new_lc)`，写入失败被完全静默忽略，导致文件虽被扫描但未绑定 SourceRoot 却仍报扫描成功。 | 彻底移除所有 `let _`，生命周期读取与写入全部通过 `?` 严格传播；写入若失败，`scan_directory` 立即返回 `Err(LoomaError::Database)`，决不虚报成功。 | `scan_suppression_e2e_test::test_scan_directory_propagates_lifecycle_persistence_error` |
| **SourceRoot 归属边界与子目录墓碑继承 (P1-A)** | **P1** | 1. `list_assets_by_prefix` 使用 `path LIKE prefix%`，导致 `D:/Games` 会误匹配 `D:/GamesBackup`。<br>2. 扫描已登记根目录的子目录时，直接创建新 SourceRoot 且不继承父目录 Untrack 墓碑，导致已 Untrack 文件被重新索引入库。 | 1. `list_assets_by_prefix` 采用 SQL 通配符转义 + 组件边界精确比对 (`path = ?1 OR path LIKE ?2/ ESCAPE '\'`)，并在内存层通过 `looma_core::is_path_under_root` 双重过滤，彻底杜绝前缀碰撞。<br>2. 扫描已登记目录的子目录时，自动识别并继承父 SourceRoot 身份；通过 `compute_relative_subpath` 将父级墓碑动态换算为子目录相对路径并注入扫描器， Untrack 文件决不复活。 | `scan_suppression_e2e_test::test_subdirectory_scan_inherits_parent_tombstones_and_boundary_isolation` |
| **MCP 真实进程级 stdio 会话 (P1-B)** | **P1** | 之前的 MCP 测试仅针对 in-process 的 `handle_json_str`，未真正拉起 `looma-mcp` 子进程并通过真实 stdin/stdout 完成标准 JSON-RPC 2.0 通信，外部 Agent 接入链路缺乏真实系统级验证。 | 新增 `mcp_subprocess_stdio_test.rs`，拉起真实 `looma-mcp` 独立子进程，通过标准输入输出完成 `initialize` $\rightarrow$ `notifications/initialized` $\rightarrow$ `tools/list` $\rightarrow$ `tools/call` 会话。证明默认 ReadOnly 拒绝写操作且数据库产生 0 副作用，`--allow-write` 参数正确放行并持久化。 | `mcp_subprocess_stdio_test::test_mcp_real_subprocess_stdio_e2e_lifecycle` |
| **桌面端打包与独立运行** | **P0** | 早期打包直接调用 `cargo build --release`，导致 Tauri v2 回退至开发配置中的 `devUrl: http://localhost:5173`，未内嵌生产前端资源，打开 `Looma.exe` 报 `ERR_CONNECTION_REFUSED`。 | 使用 `pnpm --filter @looma/desktop tauri build` 构建生产包，通过 Vite 生产编译将静态资源 (`dist/`) 嵌入独立二进制中。交付根目录独立可执行文件 `Looma.exe` (14.58 MB)，双击立即可用，完全脱离开发服务器。 | 本地桌面运行与打包验证 |

---

## 2. 深入技术实现剖析 (In-Depth Technical Details)

### 2.1 统一事务与原子回滚的备份恢复架构 (`crates/database/src/db.rs`)

在 SQLite 中，多次独立调用 Repository 方法会各自开启并提交隐式事务。若恢复执行到第 5 张表时发生约束错误，前 4 张表的数据已经写入磁盘，造成灾难性的数据库半破坏状态。

针对此问题，我们将 `restore_backup` 重构为单一原子事务执行：
```rust
// 1. 前置预校验：强制要求 manifest 与校验和
let manifest_path = target_dir.join("manifest.json");
if !manifest_path.exists() {
    return Err(LoomaError::Validation("manifest.json is required for vault restoration".into()));
}
// 校验 schema_version <= 4
if manifest.schema_version > 4 {
    return Err(LoomaError::Validation(...));
}
// 强制比对 SHA-256
let expected_checksum = manifest.checksums.get("vault_dump.json").ok_or(...)?;
// ... SHA-256 校验失败即刻终止 ...

// 2. 单一 SQLite 事务包裹全量写入
let mut conn = self.conn.lock();
let tx = conn.transaction().map_err(|e| LoomaError::Database(e.to_string()))?;

// 针对 assets, entities, memories, collections, relations, external_references,
// experiences, experience_links, source_roots, lifecycles, suppressions
// 全部使用 tx.execute(...) 写入，遇到任何失败均通过 ? 立即返回 Err
tx.commit().map_err(|e| LoomaError::Database(format!("Failed to commit restore transaction: {e}")))?;
```
**回滚证明**：新增故障注入测试 `test_backup_integrity_and_fault_injection`，在 Dump 中放置一个合法 Entity 和一个校验违规的 ExperienceEntry，向已有 1 条记录的数据库恢复。恢复因约束失败报错后，查询数据库确认只有原先的 1 条记录，后续合法的 Entity 也被 100% 干净回滚，数据库未受到任何污染。

### 2.2 路径前缀转义与子目录墓碑动态换算 (`crates/core/src/facade.rs`)

#### 1. SQL 通配符与组件边界消除前缀碰撞
在 `list_assets_by_prefix` 中：
```rust
let norm_prefix = looma_core::normalize_directory_path(prefix);
let escaped = norm_prefix.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
let slash_pattern = format!("{}/%", escaped.trim_end_matches('/'));
let exact_pattern = escaped.trim_end_matches('/').to_string();

// SQL 使用斜杠边界限定
"SELECT ... FROM assets WHERE (path = ?1 OR path LIKE ?2 ESCAPE '\\')"
```
并在取出数据后通过 `looma_core::is_path_under_root` 在内存层再次进行大小写与组件边界过滤，彻底根治了 `D:/Games` 匹配到 `D:/GamesBackup` 的潜在缺陷。

#### 2. 子目录扫描墓碑继承与归属识别
实现通用的 `compute_relative_subpath(base, target)`。当扫描子目录（如 `D:/Games/Bloodborne`）时：
1. 遍历已有 SourceRoot，自动识别其属于祖先根目录 `D:/Games`，将资产的 `root_id` 直接归属至该根目录，相对路径记录为 `Bloodborne/xxx`，避免碎片化创建重叠根目录。
2. 收集所有祖先根目录的压制墓碑（如 `Bloodborne/ebitas.exe`），通过 `compute_relative_subpath("D:/Games/Bloodborne", "D:/Games/Bloodborne/ebitas.exe")` 动态换算为子扫描相对路径 `ebitas.exe`，并压入扫描器压制列表。
3. 单元测试证明：直接对子目录触发扫描时，该文件被准确识别为 `suppressed_assets: 1` 且决不录入 `assets` 表中。

### 2.3 真实子进程 stdio 客户端接入测试 (`crates/mcp/tests/mcp_subprocess_stdio_test.rs`)

利用 Cargo 提供的环境变量 `env!("CARGO_BIN_EXE_looma-mcp")`，直接由操作系统拉起 `looma-mcp` 子进程并接管其标准输入输出管道：
1. **客户端生命周期握手**：
   - 发送 `initialize` 请求 $\rightarrow$ 读取响应校验 `serverInfo.name == "looma-mcp"`。
   - 发送 `notifications/initialized` 通知。
   - 发送 `tools/list` $\rightarrow$ 校验服务端成功注册所有可用 MCP 工具。
2. **默认只读权限门禁**：
   - 发送 `tools/call` 调用写工具 `record_experience`。
   - 读取标准输出响应：确认 `isError: true`，报错明确包含 `requires 'ContentWrite' permission, but server is running with 'ReadOnly'`。
   - 查询数据库：经验足迹总数为 0，证明未造成任何数据库副作用。
3. **真实提权写入会话**：
   - 以 `--allow-write` 启动第二个子进程，通过 stdio 先后调用 `create_work` 与 `record_experience`。
   - 确认响应成功，断开 stdio 后读取数据库，确认新作品与通关足迹已真实持久化入库。

---

## 3. 全工作区自动化测试验证汇总 (Full Test Verification)

全工作区自动化回归测试共计 **106 项测试全部一次性通过**（0 失败，0 忽略）：

```text
running 106 tests across workspace:
- looma_core (3 tests):
  test models::work::tests::test_backward_compatibility_without_type_metadata ... ok
  test models::work::tests::test_manga_and_book_type_metadata_overlays ... ok
  test models::work::tests::test_work_metadata_serde_with_type_metadata ... ok

- looma_database tests (95 tests):
  test backup_integrity_fault_test::test_backup_integrity_and_fault_injection ... ok
  test db_test::test_in_memory_db_and_services ... ok
  test experience_test (10 tests) ... ok
  test scan_suppression_e2e_test::test_scan_directory_propagates_lifecycle_persistence_error ... ok
  test scan_suppression_e2e_test::test_subdirectory_scan_inherits_parent_tombstones_and_boundary_isolation ... ok
  test scan_suppression_e2e_test::test_untrack_tombstone_scan_loop_and_lifecycle_invariants ... ok
  test source_root_lifecycle_test (7 tests) ... ok
  test work_test (73 tests) ... ok

- looma_mcp tests (5 tests):
  test test_mcp_default_is_readonly ... ok
  test test_mcp_metadata_write_permission_tier ... ok
  test test_mcp_content_write_permission_tier ... ok
  test test_mcp_full_destructive_permission_tier ... ok
  test test_mcp_real_subprocess_stdio_e2e_lifecycle ... ok

- looma_scanner tests (3 tests):
  test test_classifier ... ok
  test test_hasher ... ok
  test test_incremental_scanner ... ok

test result: ok. 106 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;
```

---

## 4. 交付清单与状态确认 (Deliverables & Status)

1. **核心库改动**：
   - `crates/core/src/facade.rs`: 增加 `compute_relative_subpath`，重写 `scan_directory` 继承父 SourceRoot 及墓碑动态换算，移除所有吞错的 `let _`。
   - `crates/database/src/db.rs`: 实现 `restore_backup` 全事务原子写入与强制 Manifest/Checksum/Schema 预校验；实现 `list_assets_by_prefix` 通配符转义与组件过滤。
2. **测试强化**：
   - `crates/database/tests/backup_integrity_fault_test.rs`: 增加中途注入损坏约束并验证数据库未被污染的事务回滚测试。
   - `crates/database/tests/scan_suppression_e2e_test.rs`: 增加子目录墓碑继承测试及生命周期写入失败错误传播测试。
   - `crates/mcp/tests/mcp_subprocess_stdio_test.rs`: 增加真实子进程 stdio 客户端通信与权限防御测试。
3. **桌面端可执行程序**：
   - 交付内嵌生产前端静态资源的单文件程序：`./Looma.exe` (14.58 MB)，可直接在 Windows 系统独立双击运行。
