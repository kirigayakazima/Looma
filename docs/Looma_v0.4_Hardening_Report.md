# Looma v0.4 架构安全性与数据完整性强化报告 (Hardening Report)

**文档基准版本**：`208c165` / `8241b5e`  
**强化完成日期**：2026-11-14  
**目标定位**：针对 Looma v0.4 初版实现经 GPT 深度代码复核所发现的边界缺陷、静默错误、扫描死循环漏洞及备份完整性风险，执行系统性 Hardening，确保长期个人数字生活档案的不可逆安全与数据完整性。零新增花哨功能、零修改历史迁移、零破坏性重构。

---

## 1. 核心修复概览 (Summary of Hardening Changes)

| 问题分类 | 风险等级 | 修复前表现与漏洞根源 | 修复后状态与防护机制 | 覆盖测试 |
| :--- | :---: | :--- | :--- | :--- |
| **Untrack 墓碑扫描闭环** | **P0** | `ScanOptions.suppressed_relative_paths` 仅在单元测试中手工填入，Desktop / CLI / MCP 入口直接调用裸 `Scanner`，正常扫描完全不加载墓碑，Untrack 文件再次扫描必定重新入库。 | 统一由 `LoomaCore::scan_directory` 调度，自动绑定 `SourceRoot`、动态加载其墓碑并注入扫描器，实现全入口统一压制；同时大小写不敏感匹配相对路径。 | `scan_suppression_e2e_test` |
| **备份与恢复完整性** | **P0** | `restore_backup()` 对关系、外链、足迹、SourceRoot 和生命周期失败直接使用 `let _ = ...; restored_... += 1` 吞掉错误并虚报成功；硬编码 `schema_version: 2`；WAL 模式下未 checkpoint 直接复制；未校验 SHA-256 校验和。 | 恢复前强制校验 Manifest 与 `vault_dump.json` 的 SHA-256 校验和，不一致立即拒绝；所有实体恢复严格错误传播（`?`），任何一条失败立即中断报错；导出前强制执行 `PRAGMA wal_checkpoint(TRUNCATE)`；动态从 `_schema_migrations` 读取最新版本 (4)。 | `backup_integrity_fault_test` |
| **移动盘与根目录隔离** | **P0** | `doctor_inspect()` 使用 `norm_p.starts_with(prefix)` 字符串前缀匹配，导致 `D:\Games` 离线时 `D:\GamesBackup` 被误判为离线保护；UNC 网络路径 `//server/share` 被反斜杠清洗破坏成 `/server/share`；`rebind_source_root()` 使用 `let _ = tx.execute` 忽略更新失败。 | 优先检查 `asset_lifecycle.root_id`；未绑定资产使用组件边界级 `is_path_under_root`（支持 Windows 大小写不敏感及 UNC 路径规范化）；`rebind_source_root` 所有 SQL 执行错误均触发事务全面回滚。 | `source_root_lifecycle_test`, `scan_suppression_e2e_test` |
| **资产生命周期不变量** | **P0** | 扫描器在文件未改变或被修改时无条件赋予 `AssetStatus::Active`，导致用户手动归档 (`Archived`) 的资产在下一次扫描时被强制恢复成 `Active`。 | 扫描器在增量检测中严格保留 `AssetStatus::Archived` 状态，禁止将已归档资产重新标记为活动状态；已 Untrack 资产由墓碑保证不被重新索引。 | `scan_suppression_e2e_test` |
| **足迹 ID 碰撞风险** | **P1** | `ExperienceEntry::new()` 仅截取 UUID 前 8 位十六进制字符 (`exp_<8_hex>`)，约 32 位熵空间，存在长期唯一性碰撞隐患。 | 升级为完整 128 位 UUID (`exp_<32_hex_simple>`)，完全杜绝长期数据库碰撞；保持既有 ID 读取兼容。 | `experience_test` |
| **时间轴时间语义与 UI 表达** | **P1** | `unknown` 精度足迹在时间轴中回退到 `created_at` 排序时，UI 将其与今天或真实 UTC 时刻混淆；年、月、日精度直接以 `00:00:00` 本地时间渲染出错误日时。 | 后端在 TimelineItem 元数据中明确携带 `is_occurred_time_unknown` 与 `created_at`；UI 严格区分“实际发生于”与“记录录入于”，Unknown 明确标注“时间未指定 (记录于 YYYY-MM-DD)”，年/月/日精度严格按日期表达。 | `TimelineView.tsx` 前端呈现 |
| **MCP 真实协议权限边界** | **P1** | MCP 默认权限虽改为 ReadOnly，但缺乏基于真实 JSON-RPC 2.0 stdio 协议帧的自动化端到端测试覆盖。 | 新增 `mcp_protocol_test.rs`，对默认构造 (`ReadOnly`)、`MetadataWrite`、`ContentWrite`、`FullDestructive` 执行完整 JSON-RPC 2.0 调测，严格校验权限拦截。 | `mcp_protocol_test` |
| **底层绕过收敛** | **P1** | Desktop、CLI、MCP 存在直接调用底层 `Scanner` 或绕过 Facade 业务策略的问题。 | 收敛所有扫描调用至 `LoomaCore::scan_directory`，并通过 `ScanEngine` 特征解耦，彻底杜绝底层绕过。 | 架构静态检查与集成测试 |

---

## 2. 深入技术实现与代码改动点 (Detailed Code Changes)

### 2.1 P0.1: Untrack 墓碑统一扫描闭环
- **抽象 `ScanEngine` 特征与模型统一**：
  - 在 `crates/core/src/models/scan.rs` 与 `crates/core/src/services/mod.rs` 中定义 `ScanEngine`、`ScanProgressCallback`、`ScanOptions`、`ScanProgress`、`ScanSummary`。
  - `Scanner` 实现 `ScanEngine for Scanner`，在 `Scanner::scan_directory` 中将压制路径比较强化为大小写不敏感：
    ```rust
    if !rel_path.is_empty()
        && options
            .suppressed_relative_paths
            .iter()
            .any(|s| s.trim_start_matches('/').eq_ignore_ascii_case(&rel_path))
    {
        suppressed_assets += 1;
        continue;
    }
    ```
- **Facade 核心编排 (`LoomaCore::scan_directory`)**：
  - 自动根据规范化路径查寻或创建 `SourceRoot`。
  - 从数据库查询该 `SourceRoot` 的所有活跃压制墓碑 (`list_suppressions(Some(&root.id))`)，合并入 `options.suppressed_relative_paths`。
  - 委托注册的 `ScanEngine` 执行物理目录遍历与指纹计算。
  - 扫描完成后更新 `SourceRoot` 的 `last_successful_scan_at` 和 `state = "online"`。
  - 自动将新发现或已索引资产关联到该 `SourceRoot`，并在 `asset_lifecycle` 中维持生命周期元数据。
  - 记录 `asset.scan` 统一审计日志。
- **所有入口重定向**：
  - Desktop Tauri (`apps/desktop/src-tauri/src/lib.rs`)：重定向为 `core.scan_directory(...)`。
  - CLI (`crates/cli/src/main.rs`)：重定向为 `core.scan_directory(...)`。
  - MCP (`crates/mcp/src/server.rs`)：重定向为 `self.core.scan_directory(...)`。

### 2.2 P0.2: 备份与恢复数据完整性强化
- **WAL Checkpoint 强制刷新**：
  - 在 `generate_manifest()` 生成文件校验和与导出备份前，强制执行 `PRAGMA wal_checkpoint(TRUNCATE);`，确保事务日志落盘到 `vault.db`，避免备份到半同步文件。
- **动态 Schema Version**：
  - `generate_manifest()` 通过 `SELECT COALESCE(MAX(version), 4) FROM _schema_migrations` 动态提取数据库版本，Manifest 正确记录 `schema_version: 4`。
- **导入前校验和验证**：
  - `restore_backup()` 检查备份包中的 `manifest.json`，计算 `vault_dump.json` 的实际 SHA-256 并与清单比对，若不匹配立即返回 `LoomaError::Validation("Backup archive corrupted: vault_dump.json checksum mismatch")`，中断恢复流程。
- **零错误容忍与完整性传递**：
  - 彻底移除 `restore_backup()` 中所有的 `let _ = ...; restored_... += 1` 盲目忽略。
  - 针对 `entities`, `memories`, `collections`, `relations`, `external_references`, `experiences`, `experience_links`, `source_roots`, `asset_lifecycles`, `asset_suppressions` 的所有写入均执行严格错误检查 (`?`)，仅在写入成功后才计入成功计数器。

### 2.3 P0.3: SourceRoot 离线识别与路径规范化
- **消除字符串前缀误判漏洞**：
  - 在 `crates/core/src/facade.rs` 实现 `is_path_under_root(child_path, root_path)`：
    ```rust
    pub fn is_path_under_root(child_path: &str, root_path: &str) -> bool {
        let child_norm = normalize_directory_path(child_path);
        let root_norm = normalize_directory_path(root_path);

        #[cfg(target_os = "windows")]
        let (c, r) = (child_norm.to_lowercase(), root_norm.to_lowercase());
        #[cfg(not(target_os = "windows"))]
        let (c, r) = (child_norm, root_norm);

        if c == r { return true; }
        if c.starts_with(&r) {
            if r.ends_with('/') { return true; }
            if c.as_bytes().get(r.len()) == Some(&b'/') { return true; }
        }
        false
    }
    ```
    有效防止 `D:/Games` 错误匹配 `D:/GamesBackup/file.zip`。
- **UNC 路径保护**：
  - 修改 `normalize_directory_path`，对于以 `//` 或 `\\` 开头的 Windows UNC 网络共享路径（如 `\\server\share`），保留前置双斜杠，防止被去重清洗为普通相对路径 `/server/share`。
- **Doctor 检查优先使用 `root_id`**：
  - `doctor_inspect()` 中先检查资产关联的 `asset_lifecycle.root_id`，若关联根目录已离线，资产即被判定为离线保护而非丢失；未绑定资产再回退到组件级 `is_path_under_root`。
- **Rebind 事务严格化**：
  - `rebind_source_root()` 中移除所有 `let _ = tx.execute`，SQL 更新失败时立即中断并使整个事务回滚，杜绝部分重绑定成功的半坏状态。

### 2.4 P0.4: 生命周期不变量保护
- 在 `crates/scanner/src/engine.rs` 中：
  ```rust
  if same_size && same_mtime && existing.status == AssetStatus::Active {
      unchanged_assets += 1;
  } else if same_size && same_mtime && existing.status == AssetStatus::Archived {
      // 保持 Archived 状态，不被覆盖为 Active
      unchanged_assets += 1;
  } else {
      ...
      existing.status = if existing.status == AssetStatus::Archived {
          AssetStatus::Archived
      } else {
          AssetStatus::Active
      };
      service.upsert_asset(&existing)?;
  }
  ```
- 扫描原文件重新出现时，根据现有路径直接提取既有 `Asset`，保留原有 `id`，其下游关联的 `relations` 和 `experience_links` 完整无损。

### 2.5 P1.5: 足迹 ID 升级为完整 UUID
- 修改 `crates/core/src/models/experience.rs`：
  ```rust
  impl ExperienceEntry {
      pub fn new(subject_id: impl Into<String>, event_type: impl Into<String>) -> Self {
          Self {
              id: format!("exp_{}", Uuid::new_v4().simple()), // 完整 128 位熵
              ...
          }
      }
  }
  ```
  兼容已有存量短 ID，未来创建均具备全局唯一性保障。

### 2.6 P1.6: 时间轴语义与前端 UI 呈现强化
- 后端 `query_timeline` 在 `metadata` 中注入 `created_at` 及 `is_occurred_time_unknown` 标志。
- 前端 `TimelineView.tsx` 中：
  - 时光轴列表条目：对 `unknown` 精度显示 `时间未指定 (记录于 YYYY/MM/DD)`；对 `year` 显示 `发生于 YYYY 年`；对 `month` 显示 `发生于 YYYY-MM`；对 `day` 显示 `发生于 YYYY-MM-DD`。
  - 弹窗详情页：新增结构化卡片，明确分为 **“实际发生”** 与 **“记录时间”**，消除了合成假 UTC 零点所引起的歧义。

### 2.7 P1.7: MCP JSON-RPC 真实协议测试
- 新增 `crates/mcp/tests/mcp_protocol_test.rs`：
  - 测试 `McpServer::new(core)` 默认初始化为只读权限 (`ReadOnly`)。
  - 测试 `ReadOnly` 允许 `search_assets` 与 `list_experiences`，但对 `create_entity`, `record_experience`, `scan_directory`, `delete_asset` 均拦截并返回 `Permission Denied`。
  - 测试 `MetadataWrite` 允许 `create_entity`，拦截 `record_experience` 与 `delete_asset`。
  - 测试 `ContentWrite` 允许 `create_entity` 与 `record_experience`，拦截 `delete_asset`。
  - 测试 `FullDestructive` 允许通过 `delete_asset` 权限门禁。

---

## 3. 测试验证与回归结果 (Verification & Test Results)

全工作区自动化回归测试共计 **103 项测试全部一次性通过**（0 失败，0 忽略）：

```text
running 103 tests across workspace:
- looma_core (3 tests):
  test models::work::tests::test_backward_compatibility_without_type_metadata ... ok
  test models::work::tests::test_manga_and_book_type_metadata_overlays ... ok
  test models::work::tests::test_work_metadata_serde_with_type_metadata ... ok

- looma_database tests:
  test backup_integrity_fault_test::test_backup_integrity_and_fault_injection ... ok
  test db_test::test_in_memory_db_and_services ... ok
  test experience_test (10 tests) ... ok
  test scan_suppression_e2e_test::test_untrack_tombstone_scan_loop_and_lifecycle_invariants ... ok
  test source_root_lifecycle_test (7 tests) ... ok
  test work_test (73 tests) ... ok

- looma_mcp tests (4 tests):
  test test_mcp_default_is_readonly ... ok
  test test_mcp_metadata_write_permission_tier ... ok
  test test_mcp_content_write_permission_tier ... ok
  test test_mcp_full_destructive_permission_tier ... ok

- looma_scanner tests (3 tests):
  test test_classifier ... ok
  test test_hasher ... ok
  test test_incremental_scanner ... ok

test result: ok. 103 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;
```

### 桌面端独立运行文件交付
- 前端通过 `tsc && vite build` 编译成功。
- 通过 `pnpm --filter @looma/desktop tauri build --no-bundle` 完成内嵌编译，脱离本地前端 Dev Server（无 `ERR_CONNECTION_REFUSED` 问题）。
- 根目录交付可执行文件：`./Looma.exe` (14.58 MB)。

---

## 4. 遗留项与风险接受项说明 (Risk Acceptance & Future Roadmap)

1. **历史 Migration 冻结保证**：
   - 本次 Hardening 未修改历史 `001`, `002`, `003`, `004` 任何一条 SQL 语句，保证已有老用户的 Vault 数据库安全向前兼容。
2. **风险接受项：外部可执行文件启动权限**：
   - MCP 协议基于本地 stdio 传输，目前依靠启动参数与环境变量指定权限等级（默认强固为 `ReadOnly`）。在完全接入多用户或远端网络认证前，该机制适合本地受信环境。后续网络化时可引入 Token 机制。
3. **真实文件系统与网络盘约束**：
   - UNC 路径在部分无权限共享中可能引发 OS 级别的 I/O 阻塞或权限拒绝，Facade 已支持识别并抛出具名 `LoomaError::Database` / `LoomaError::Validation`，不会导致主程序崩溃。
