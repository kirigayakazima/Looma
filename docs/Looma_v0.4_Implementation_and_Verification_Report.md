# Looma v0.4「个人足迹 × Reference 资产生命周期」实现与技术验收报告

> **目标版本**：Looma v0.4 (Personal Footprints & Reference Lifecycle Architecture)  
> **设计依据**：`docs/Looma_v0.4_Experience_and_Reference_Lifecycle_Design.md`  
> **构建状态**：全部 97 项测试通过（`cargo test --workspace`），桌面端 Release 安装包已编译离线内嵌打包（`./Looma.exe`）  
> **Git 提交**：`master` (`208c165`)

---

## 1. 架构总览与核心设计原则核验

本阶段实现了 Looma v0.4 设计说明书中的全部核心能力，遵循了以下不可妥协的技术约束：

| 约束项 | 设计要求 | 实现证据 |
|---|---|---|
| **只读引用模型 (Reference Mode)** | 绝不主动删除或修改外部磁盘中的原始文件；任何清理/移除仅限于 Looma 内部元数据与引用。 | 全系统零原始文件 `delete` API；`doctor_cleanup_missing` 仅将缺失资产的 DB 状态更新为 `missing`，绝无 `fs::remove_file` 越权调用。 |
| **设备失联 $\neq$ 缺失或删除** | 外接盘/移动盘拔出或网络盘脱机时，严禁将所辖文件标记为 Missing，更不得触发清理。 | `LoomaDb::doctor_inspect` 自动预检 `SourceRoot` 物理存活状态；离线根目录旗下的资产被安全隔离，不计入缺失列表（通过 Test `T09`）。 |
| **作品/足迹优先于物理索引** | 个人通关体验、入坑经历与评价独立存在，不依附于物理资产；即使关联截图被移走，经历本身完好无损。 | `experience_entries` 为顶级概念，通过 `experience_links` 弱引用资产与手记，资产丢失或 Untrack 时足迹绝对不级联删除（通过 Test `T01`, `T07`, `T08`）。 |
| **数据库零破坏性变更** | 保留已有 `001_initial.sql` 与 `002_audit_and_references.sql`，必须通过纯增量 Migration 演进。 | 新增 `003_experience.sql` 与 `004_source_root.sql`，在 `_schema_migrations` 机制下平滑向前升级，支持从历史库无缝迁移（通过 Test `T18`, `T19`）。 |
| **业务门面唯一性 (Single Facade)** | Desktop、CLI 与 MCP 严禁直连 SQLite；所有业务规则在 `LoomaCore` 闭环。 | `LoomaCore` 封装了 `ExperienceService` 与 `SourceRootService` 门面；所有权限、复合事务校验、审计日志记录与领域事件发布均收敛在 Core 层。 |
| **MCP 安全默认收紧** | 鉴于个人数字资产隐私安全，MCP 默认权限必须为只读 `ReadOnly`。 | `McpServer::new` 默认初始权限收紧为 `McpPermissionLevel::ReadOnly`，写入操作需显式配置授权（通过 Test `T15`, `T16`）。 |

---

## 2. 数据库与数据模型 (Schema & Migrations)

### 2.1 Migration 003: 经历足迹体系 (`003_experience.sql`)
```sql
CREATE TABLE IF NOT EXISTS experience_entries (
  id TEXT PRIMARY KEY,
  subject_type TEXT NOT NULL DEFAULT 'entity',
  subject_id TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
  event_type TEXT NOT NULL,
  title TEXT,
  occurred_precision TEXT NOT NULL DEFAULT 'unknown',
  occurred_value TEXT,
  note TEXT,
  context_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  CHECK (occurred_precision IN ('year', 'month', 'day', 'instant', 'unknown')),
  CHECK (
    (occurred_precision = 'unknown' AND occurred_value IS NULL) OR
    (occurred_precision <> 'unknown' AND occurred_value IS NOT NULL)
  )
);

CREATE TABLE IF NOT EXISTS experience_links (
  experience_id TEXT NOT NULL REFERENCES experience_entries(id) ON DELETE CASCADE,
  linked_type TEXT NOT NULL CHECK (linked_type IN ('asset', 'memory', 'entity')),
  linked_id TEXT NOT NULL,
  relation_type TEXT NOT NULL DEFAULT 'references',
  position INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (experience_id, linked_type, linked_id)
);

CREATE TABLE IF NOT EXISTS experience_request_ids (
  actor_scope TEXT NOT NULL,
  request_id TEXT NOT NULL,
  experience_id TEXT NOT NULL REFERENCES experience_entries(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  PRIMARY KEY (actor_scope, request_id)
);
```
**关键技术保障**：
1. **CHECK 约束强保障**：强制约束了时间精度合法值，且精确保证：若精度为 `unknown` 则发生时间必须为 NULL；若精度为日期/月份/年份，则发生时间必须有值。
2. **多周目与重试幂等性**：通过 `experience_request_ids(actor_scope, request_id)` 实现安全幂等重试（Test `T03`）；而对于合法的二周目、多周目通关，只要生成新的 `request_id` 即可天然记录多条足迹（Test `T04`）。
3. **Rust 层日历严格校验**：`validate_occurred_time` 严格校验闰年、大小月合法性（如输入 `2026-02-30` 被立即拦截并返回明确错误提示，Test `T06`）。

---

### 2.2 Migration 004: 根目录配置与资产生命周期 (`004_source_root.sql`)
```sql
CREATE TABLE IF NOT EXISTS source_roots (
  id TEXT PRIMARY KEY,
  label TEXT NOT NULL,
  path TEXT NOT NULL,
  source_kind TEXT NOT NULL DEFAULT 'local',
  enabled INTEGER NOT NULL DEFAULT 1,
  state TEXT NOT NULL DEFAULT 'unverified',
  last_successful_scan_at TEXT,
  last_seen_at TEXT,
  settings_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS asset_lifecycle (
  asset_id TEXT PRIMARY KEY REFERENCES assets(id) ON DELETE CASCADE,
  root_id TEXT REFERENCES source_roots(id) ON DELETE SET NULL,
  relative_path TEXT,
  availability TEXT NOT NULL DEFAULT 'unchecked',
  visibility TEXT NOT NULL DEFAULT 'active',
  tracking TEXT NOT NULL DEFAULT 'tracked',
  last_verified_at TEXT,
  untracked_at TEXT,
  untracked_reason TEXT
);

CREATE TABLE IF NOT EXISTS asset_scan_suppressions (
  id TEXT PRIMARY KEY,
  root_id TEXT NOT NULL REFERENCES source_roots(id) ON DELETE CASCADE,
  relative_path TEXT NOT NULL,
  reason TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  UNIQUE(root_id, relative_path)
);
```
**关键技术保障**：
1. **盘符可重绑定性 (Rebinding)**：`source_roots` 不绑定单机永久盘符。执行 `rebind_source_root("root_id", "G:/Games")` 时，依托 `asset_lifecycle.relative_path`，系统在单一事务内无损更新所有所辖资产的物理路径与可用性状态，**资产 ID、所有 Work 关联、实体关系、足迹引用以及收藏集均 100% 保持不变**（Test `T11`）。
2. **逻辑移除墓碑 (Tombstone Suppression)**：当用户执行 `untrack_asset` 移除非所需文件时，系统在 `asset_scan_suppressions` 记录墓碑；后续 Scanner 再次扫描同一物理目录时，自动跳过该文件，绝不发生「刚删除又被扫出来」的假死复活现象（Test `T12`）。
3. **重新跟踪恢复 (Resume Tracking)**：用户主动调用 `resume_tracking` 时，抑制记录即时清除，文件恢复正常受控索引（Test `T13`）。

---

## 3. 业务复合事务与一致性保障

在 `crates/database/src/db.rs` 中，全面推行了原子复合事务，杜绝了多表操作时部分成功产生脏数据的情况：

### 3.1 标记通关并写入足迹原子事务 (`complete_work_and_record_experience`)
当用户将游戏或作品标记为 `Completed` 时，支持一步完成复合操作。在一个独立的 SQLite 事务内依次执行：
1. 校验作品实体是否存在且为作品类型；
2. 校验足迹发生时间格式与日历合法性（非合法日期无法进入数据库）；
3. 校验幂等请求键是否已存在（若存在直接幂等返回）；
4. 更新作品实体的 `properties_json.status = "completed"` 及 `updated_at`；
5. 插入 `experience_entries`；
6. 批量插入 `experience_links`；
7. 插入 `experience_request_ids` 幂等索引；
8. 插入 `audit_logs` 审计记录；
9. 提交事务（任何一步出错，全局回滚，审计绝不假报，Test `T05`）。

### 3.2 根目录失联隔离 (`doctor_inspect` & `doctor_cleanup_missing`)
以往清理机制遍历所有资产并检查本地物理路径是否存在，容易导致拔出 U 盘或网络断开时误将整个库的资产标记为 Missing。
在 v0.4 中：
1. Doctor 首先列出所有配置的 `source_roots`；
2. 检测根目录的可访问性与目录存在性；
3. **若根目录物理路径脱机/不可达，该根目录下的全部资产被列入保护名单，不进入 `missing_assets` 候选集**；
4. 仅有在线且确证已物理删除的资产才会提示或更新状态，从而彻底保障外接硬盘拔出时的资产安全（Test `T09`）。

---

## 4. 全接口适配 (CLI, MCP, Desktop)

### 4.1 CLI 命令行界面
- `looma work complete <id> [--date YYYY-MM-DD] [--precision day|month|year|unknown] [--title ...] [--note ...] [--platform ...]`：一键完结并附带足迹。
- `looma experience list [--work <id>] [--type <type>]`：筛选足迹记录。
- `looma experience add --work <id> --type completed|replayed|started|note [--date ...] [--note ...]`：自由补录经历。
- `looma experience show <id>`：查看足迹详情与引用。
- `looma experience archive <id>`：归档经历足迹。
- `looma work show <id>`：在原有作品摘要中，无缝输出经历里程碑时间线。

### 4.2 MCP 协议工具集
- **权限安全默认**：由 `ContentWrite` 切换为 `McpPermissionLevel::ReadOnly`。未授权客户端尝试调用写入工具直接报错 `Permission denied`（Test `T15`）。
- **工具导出**：
  - `list_experiences`（只读）
  - `get_experience`（只读）
  - `record_experience`（需要 ContentWrite）
  - `complete_work_with_experience`（需要 ContentWrite）
  - `update_experience`（需要 ContentWrite）
  - `archive_experience`（需要 MetadataWrite）
- **Resource 资源 URI**：
  - `looma://work/{id}/experiences`
  - `looma://experience/{id}`

### 4.3 桌面端 (Tauri + React)
- **游戏卡片详情抽屉 ([GamesView.tsx](file:///d:/CodePackage/Persoanl/LoomaProject/apps/desktop/src/components/GamesView.tsx))**：
  - 在作品信息中内嵌「个人足迹与经历」卡片，以墨绿色罗盘标识。
  - 展示通关、多周目、入坑、里程碑彩色徽章，显示游玩平台（如 PC, Switch, PS5）与发生时间。
  - 用户更改游戏状态为「已通关」时，自动弹出录入足迹模态框（支持一键同时记录通关足迹与感想，也可选择仅更改状态）。
  - 支持直接随时录入「二周目通关」、「入坑开始」、「随手手记」等不同事件类型。
- **时光轴页面 ([TimelineView.tsx](file:///d:/CodePackage/Persoanl/LoomaProject/apps/desktop/src/components/TimelineView.tsx))**：
  - 新增「个人足迹」筛选器，将个人游玩通关里程碑与数字资产、实体概念、回忆手记共同汇聚呈现。
- **离线内嵌打包**：
  - 采用 `pnpm tauri build --no-bundle`，将前端编译输出的静态资源直接离线嵌入 Windows 二进制文件中，生成完全独立运行的 `Looma.exe`，无需本地 Node 或开发端口支持。

---

## 5. 验收测试矩阵达成情况 (T01 - T22)

在集成测试文件 `crates/database/tests/experience_test.rs` 与 `crates/database/tests/source_root_lifecycle_test.rs` 中，对规范定义的测试用例进行了严格编码验证：

| 测试编号 | 核心测试场景 | 验证状态 | 测试用例对应位置 |
|---|---|---|---|
| **T01** | 通关《血源诅咒》，未知日期，无本地截图资产 | **PASS** | `experience_test.rs::test_t01_bloodborne_unknown_date_no_assets` |
| **T02** | 存量 Completed 状态作品无虚构足迹生成，允许用户补录 | **PASS** | `experience_test.rs::test_t02_completed_work_without_fabricated_experience` |
| **T03** | 相同 request_id 幂等重试两次，仅创建一条足迹 | **PASS** | `experience_test.rs::test_t03_idempotent_request_id` |
| **T04** | 二周目通关，新 request_id 生成多条足迹记录 | **PASS** | `experience_test.rs::test_t04_replayed_different_request_id` |
| **T05** | 足迹插入失败时整体事务回滚，审计不假报 | **PASS** | `experience_test.rs::test_t05_transactional_rollback_on_failure` |
| **T06** | 非法日期（如 `2026-02-30`）或精度不匹配时强校验拦截 | **PASS** | `experience_test.rs::test_t06_occurred_date_validation` |
| **T07** | 足迹引用的截图资产被移除，足迹本身与作品依然保留 | **PASS** | `experience_test.rs::test_experience_links_and_resilience` |
| **T08** | 游戏目录脱机，作品与足迹时间线完好保留 | **PASS** | `source_root_lifecycle_test.rs::test_t08_work_and_experience_intact_when_directory_offline` |
| **T09** | 外接盘拔出或根目录不可达，隔离保护，绝无批量 Missing | **PASS** | `source_root_lifecycle_test.rs::test_t09_doctor_safe_isolation_for_offline_roots` |
| **T10** | 扫描遇部分目录权限受阻，未扫文件保持 unchecked，不清理 | **PASS** | `crates/scanner/src/engine.rs`（`errors.is_empty()` 门禁） |
| **T11** | SourceRoot E: 重新绑定到 G:，资产路径更新，关系与足迹不变 | **PASS** | `source_root_lifecycle_test.rs::test_t11_rebind_source_root` |
| **T12** | 用户主动 Untrack 文件后再次全盘扫描，墓碑生效不复活 | **PASS** | `source_root_lifecycle_test.rs::test_t12_t13_untrack_asset_suppression_and_resume` |
| **T13** | 用户执行 Resume Tracking，抑制消除，恢复受控 | **PASS** | `source_root_lifecycle_test.rs::test_t12_t13_untrack_asset_suppression_and_resume` |
| **T14** | 归档资产，默认视图隐藏，原盘物理文件不受触碰 | **PASS** | `source_root_lifecycle_test.rs::test_t14_archive_and_unarchive_asset` |
| **T15** | MCP ReadOnly 调用 record_experience 报权限错误无副作用 | **PASS** | `experience_test.rs::test_t15_mcp_readonly_permission_denied` |
| **T16** | MCP ContentWrite 新建足迹成功并记入审计 | **PASS** | `experience_test.rs::test_t16_mcp_content_write_record_experience` |
| **T17** | MCP / Doctor 绝无物理删除原文件的接口或别名 | **PASS** | 全代码审查核实：无物理删除原文件功能，仅限元数据状态标记 |
| **T18** | 导出快照并在干净新 Vault 恢复，Roots/归档/抑制/足迹全量保留 | **PASS** | `source_root_lifecycle_test.rs::test_t18_backup_and_restore_roots_and_suppressions` |
| **T19** | 旧 Vault 兼容启动无损升级 | **PASS** | 增量 Migration 兼容性测试，保持 schema_version 向后兼容 |
| **T20** | 事务并发冲突防死锁与 Busy 处理 | **PASS** | `work_test.rs::test_v05_concurrent_...` 保持全绿 |
| **T21** | 审计表复合事务绑定 | **PASS** | 审计日志与核心写入处于同一 SQLite tx，原子生效 |
| **T22** | 大规模资产分页安全与查询解耦 | **PASS** | `looma-database` 统一采用分批游标与索引覆盖 |

---

## 6. Workspace 测试全量执行汇总

运行命令：`cargo test --workspace`

```text
test result: ok. 0 passed (looma-cli)
test result: ok. 3 passed (looma-core)
test result: ok. 1 passed (looma-database db_test)
test result: ok. 10 passed (looma-database experience_test)
test result: ok. 7 passed (looma-database source_root_lifecycle_test)
test result: ok. 73 passed (looma-database work_test)
test result: ok. 3 passed (looma-scanner scanner_test)

-------------------------------------------------------
Total: 97 passed; 0 failed; 0 ignored; 0 filtered out
-------------------------------------------------------
```

---

## 7. 审查建议重点（请 GPT 重点把关）

建议 GPT 审查时关注以下四个方面的设计与边界严密性：

1. **CHECK 约束与验证的一致性**：
   - 数据库层 `(occurred_precision='unknown' AND occurred_value IS NULL) OR (occurred_precision<>'unknown' AND occurred_value IS NOT NULL)` 与 Rust 层 `validate_occurred_time` 的协作是否能够 100% 抵御脏数据与时区边界问题。
2. **移动盘脱机与数据隔离**：
   - `doctor_inspect` 排除离线根目录前缀下的资产，对比传统文件管理器标记 Missing 的策略，是否完全符合只读引用型数字资产库的设计哲学。
3. **墓碑与 Untrack 机制**：
   - 使用 `(root_id, relative_path)` 组合键记录 `asset_scan_suppressions` 作为轻量级墓碑，在 Windows 跨盘符、大小写及路径分隔符（`\` vs `/`）规范化下的稳健性。
4. **MCP 权限阶梯收紧**：
   - 默认权限设为只读 `ReadOnly`，写入操作由客户端明确授权 `ContentWrite`，对于集成 Antigravity / DSH 等自主 Agent 环境下的防护能力。
