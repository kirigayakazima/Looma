# Looma v0.4 详细设计：个人足迹 × Reference 资产生命周期

> **文档类型**：可执行架构设计 / Antigravity 开发任务基线  
> **版本**：v0.4-draft.1  
> **编制日期**：2026-11-14  
> **开发目标**：在保留现有 Looma 功能的前提下，把它从“作品和文件索引工具”升级为“可持续记录个人数字生活的本地优先系统”。  
> **优先原则**：数据安全 > 语义正确 > 使用负担低 > 可扩展性 > UI 功能数量。  
> **执行要求**：本文件提供目标与候选实现。反重力必须以**当前本地仓库 HEAD**核对 API、Schema、迁移编号，再提出差异与小批次实施方案；不得直接把本文件中的示意代码当成已存在的 API。

---

## 0. 给开发 Agent 的首要指令（必须阅读）

1. 先阅读本文件，再阅读项目现存 README、架构文档、最新迁移和测试，不得从零重写。
2. 对当前仓库执行 `git status`、记录 HEAD 与未提交改动。**不得覆盖用户的未提交工作**；先建独立开发分支，修改前建立可验证的 Vault 备份。
3. 以 `crates/core/src/facade.rs` 的 `LoomaCore` 为唯一业务入口；桌面、CLI、MCP 不直接操作业务 SQL。
4. **严禁使用“删除扫描失联文件”代替“从 Looma 移除”**。源盘不可达时不能触发数据清理。
5. 对已有 Migration **只新增，不修改**。不得修改 `001_initial.sql` 或已执行的迁移脚本。
6. 先完成「足迹最小闭环」，再完成「来源根目录与资产生命周期」。两部分可以独立验收；不要一次提交所有变更。
7. 默认不调用云服务、不移动或删除原始媒体，不运行游戏模拟器、不自动录屏、不引入 LLM/OCR/向量数据库。
8. 每阶段提供：实际文件改动、迁移编号、接口变化、测试命令与结果、演示路径、风险和回滚步骤。只宣称测试实际证明的内容。
9. 如果发现本文设计与现有数据约束冲突，提出 ADR（Architecture Decision Record）说明取舍，不要默默替换设计。

---

## 1. 产品目标和反目标

### 1.1 一句话定位

**Looma 不负责强迫用户整理硬盘，而是把分散的文件、作品、项目和人生经历联系起来。**

用户可以继续将游戏放在 D 盘、视频放在 E 盘、模型放在 F 盘，不改变目录；Looma 通过引用形成可搜索、可关联、可回忆、可备份的结构化档案。

### 1.2 三个相互独立的领域

| 概念 | 负责回答 | 典型实例 | 删除关系 |
|---|---|---|---|
| **Asset（资产）** | 文件或目录是什么、在哪里、是否可访问？ | 游戏目录、截图、音频、权重、工程文件 | 索引移除不等于删除原文件 |
| **Subject（档案对象）** | 它属于哪款游戏、哪部动画、哪个项目？ | `Work` 游戏、动漫、书籍；`Entity` 项目和设备 | 可无任何本地文件而存在 |
| **Experience（足迹）** | 我与这个对象发生了什么？ | 通关游戏、看完动画、发布项目、完成一次声音合成 | 原始文件删掉或卸载，足迹仍保留 |

现有 `Memory` 是“自由文本/感想”，`Experience` 是“发生过的结构化事实”；两者**可关联但不互相替代**。

### 1.3 用户真实场景（设计验证用）

- **《血源诅咒》**：PC 模拟器版本已通关，可能没有截图、没有 Steam 成就，也不确定具体日期。必须可以补录“通关”且不制造虚假的精确日期。
- **Lost Ruins / 其他游戏**：有时处于游玩中、暂停、二周目；需要多个通关/重玩足迹，不让 `Completed` 状态覆盖历史。
- **动漫/漫画/小说**：继续使用原有步进器更新日常进度；仅明确完成、重温、特别里程碑进入个人足迹。
- **Looma / LucidHeart / DSHA 项目**：仓库链接、版本发布、设备、备份和工程产物可以相关联；不把 Looma 变成任务管理器。
- **RVC 音频实验**：本地模型权重、原音、输出音频路径杂乱；通过引用和关系聚合，保存“某次满意的合成”这一经历。
- **移动硬盘/NAS**：断线 ≠ 删除；更换盘符 ≠ 新文件；扫描不到 ≠ 用户主动放弃。

### 1.4 不做的事

v0.4 不做：游戏平台成就同步、自动游戏时间追踪、后台截图/录像、模拟器操控、全盘自动整理/移动、原文件永久删除、全自动无确认智能分类、云同步、插件市场、内置 AI 聊天。

---

## 2. 基于已查公开源码的现状映射（不是本地测试结论）

在 2026-11-14 查阅 `master` 分支公开源码，已看到：

| 已有模块 | 位置 | 复用方案 |
|---|---|---|
| `WorkMetadata`, `WorkType`, `RecordStatus` | `crates/core/src/models/work.rs` | 作品状态继续使用原模型；避免重复建游戏表 |
| `LoomaCore`, `VaultRepository` | `crates/core/src/facade.rs` | 增加 Experience 业务 API 与事务边界 |
| `Asset`, `AssetStatus` | `crates/core/src/models/asset.rs` | 兼容 `Active/Missing/Archived`；可用性与归档最终拆分 |
| `AssetService`, `EntityService`, `TimelineService`, `BackupService` | `crates/core/src/services/mod.rs` | 扩展仓储 Trait；不让 UI 直连 DB |
| `LoomaDb`, SQLite WAL | `crates/database/src/db.rs` | 新增迁移与 Repository 实现 |
| 现有 Migration 注册 | `crates/database/src/migration.rs` | 目前公开版注册 `001` 与 `002`；本地 HEAD 重新确认下一编号 |
| `GamesView.tsx` | `apps/desktop/src/components/GamesView.tsx` | 作品详情中增加“个人足迹”并接入完成工作流 |
| `McpServer`, permission enum | `crates/mcp/src/server.rs` | 增加可控 Experience 工具；**核查默认权限** |
| `query_timeline()` | `crates/database/src/db.rs` | 增加 `experience` 项来源，支持分页和过滤 |

**已识别的风险**：当前公开版 `McpServer::new()` 默认 `ContentWrite`，对未来可写用户体验记录的场景过于宽松；建议将未明确配置的客户端默认限制为 `ReadOnly`，检查 CLI 入口是否绕过。现有 Core Audit 在部分路径中位于数据库写入**之后**，新增“完成 + 足迹 + 审计”的复合事务时不可照抄其非原子顺序。

**未验证**：本地真实 HEAD、所有测试结果、最新数据库迁移及 UI 演示运行情况。开发者必须重新检查。

源码参考（只作核对入口，后续应以本地代码为准）：
- https://github.com/kirigayakazima/Looma
- https://raw.githubusercontent.com/kirigayakazima/Looma/master/crates/core/src/models/work.rs
- https://raw.githubusercontent.com/kirigayakazima/Looma/master/crates/core/src/facade.rs
- https://raw.githubusercontent.com/kirigayakazima/Looma/master/crates/database/src/db.rs
- https://raw.githubusercontent.com/kirigayakazima/Looma/master/crates/core/src/models/asset.rs
- https://raw.githubusercontent.com/kirigayakazima/Looma/master/crates/mcp/src/server.rs

---

# Part A — Experience：个人足迹系统

## 3. ADR-001：足迹必须是第一类持久化实体

### 3.1 不采用的方案

- **只在 Work 中增加 `completed_at`**：无法记录二周目、多次完结、作品之外的里程碑。
- **只写 Markdown Memory**：无法可靠检索“首次通关、二周目、完成日期”等结构化数据，使用负担高。
- **只依赖 DomainEvent / Audit**：它们是技术事件，不是长期用户记忆；会受到日志保留、权限和业务回放影响。
- **只修改当前 `Work.status`**：状态是现在，足迹是历史；不能相互覆盖。

### 3.2 采用方案

新增 `ExperienceEntry`。默认支持 `subject_type=entity`（覆盖 Work 与项目实体）；将来允许 `subject_type=asset` 或 `external_reference`，但 MVP 先严格限定到 `entity` 以确保关联一致性。

```text
Entity / Work
 ├─ current status / progress / rating
 ├─ ExperienceEntry 1 [completed]
 │    ├─ optional Memory note
 │    ├─ optional referenced screenshot(s)
 │    └─ optional contextual metadata
 ├─ ExperienceEntry 2 [replayed]
 └─ local Assets / ExternalReferences / Collections
```

**基本不变量**：任何 `ExperienceEntry` 的存在都不得依赖当前能否打开本地文件。

## 4. ExperienceEntry 模型与时间语义

### 4.1 Rust 领域模型（示意，不是直接粘贴必编译代码）

```rust
pub struct ExperienceEntry {
    pub id: String,                    // exp_<uuid>，稳定标识
    pub subject_type: String,          // MVP: "entity"
    pub subject_id: String,            // Work 实际为 Entity ID
    pub event_type: String,            // completed/started/milestone/replayed/...
    pub title: Option<String>,         // 用户自定义；默认可按事件生成展示文案
    pub note: Option<String>,          // 可选短感想，长篇放 Memory
    pub occurred_value: Option<String>,// "2026", "2026-11", "2026-11-14" 或带时区时间
    pub occurred_precision: String,    // unknown/year/month/day/instant
    pub context_json: serde_json::Value,// platform, environment, run_number 等
    pub visibility: String,            // normal/archived（仅隐藏，不销毁）
    pub created_at: DateTime<Utc>,     // 实际入库时间，永远有值
    pub updated_at: DateTime<Utc>,
}
```

**日期处理必须正确**：
- “最近通关，不记得哪一天” → `occurred_precision=unknown`, `occurred_value=NULL`，**不应擅自填写当前日期为通关日**。
- “2026 年 11 月通关” → `month` + `2026-11`。
- “11 月 14 日通关” → `day` + `2026-11-14`。
- 精确发生时间 → `instant` + RFC3339（必须带时区）。
- UI 默认日期可以建议“今天”，但必须明确是用户确认的日期；导入/AI 创建时不得自动假设。
- `created_at` 是录入时间，与实际发生时间严格分开。
- 排序：准确时间优先依据 `occurred_value`；未知时间使用 `created_at` 作为**界面排序回退**，必须显示“发生时间未填写”，不得伪装真实发生日。

> 数据库里建议增加一列派生 `occurred_sort_at`（用于索引排序）或按日期精度构造排序键；不要仅靠字符串混合时间格式直接做时间范围比较。日期字段应由 Core 验证。

### 4.2 事件类型

| 标准 key | 展示 | 例子 |
|---|---|---|
| `started` | 开始 | 开始游玩游戏、开始读书 |
| `progress_milestone` | 里程碑 | 打败一个难忘 Boss，读到一个重要章节 |
| `completed` | 完成/通关 | 游戏首次通关，读完书，完成项目版本 |
| `replayed` | 重温/二周目 | 二周目通关，重看动画 |
| `published` | 发布 | 发布软件版本、发布作品 |
| `created` | 创作完成 | RVC 成品、图片/音乐创作 |
| `reflection` | 回顾 | 某次回顾/总结 |
| `custom:<slug>` | 自定义 | 为未来插件留余地，受 Core 校验 |

事件类型**不能与 WorkType 绑定**；不要求一个游戏事件类型在项目上也有业务意义。UI 可以按对象类型提供不同模板，底层仍使用同一模型。

### 4.3 关联资产与手记

MVP 建议独立 `experience_links`：

```text
experience_id
linked_type     // "asset" | "memory" | "external_reference"
linked_id
relation_type   // evidence | note | reference
position
```

好处：类型明确、容易查询和导出，不需要修改现有泛化 `Relation` 数据假设；若当前 Relation 系统已经完全支持 Experience 作为节点，可复用它，但必须通过集成测试验证增删和跨类型关系，不允许重复维护两套相同链接。

支持零附件、零评语：**“通关”事实必须可以单独保存**。

### 4.4 例子：《血源诅咒》历史补录

```json
{
  "subject_type": "entity",
  "subject_id": "<existing-work-id>",
  "event_type": "completed",
  "title": "完成首次通关",
  "note": null,
  "occurred_value": null,
  "occurred_precision": "unknown",
  "context_json": {
    "platform": "PC",
    "environment": "emulator",
    "playthrough": 1
  }
}
```

其中 `subject_id` 必须解析已有 Work ID，不能由名称随意生成，也不需要 Steam 成就或录像证明。

## 5. Work 状态与 Experience 的关系

当前作品状态和足迹属于不同维度：

```text
Work.status: in_progress / completed / paused / dropped / revisit / ...
Work.progress: 当前进度（高频、可反复覆盖）
ExperienceEntry: 不可被状态更新覆盖的历史记录（低频）
```

### 5.1 关键操作：`complete_work_and_record_experience`

用户在游戏详情点击「完成通关」时：

1. 校验 Work 存在、是作品实体、调用者有权限。
2. 读取旧状态；如果旧状态不是 `completed`，更新 `Work.status=completed`。
3. 创建一条 `completed` 体验，录入已确认的发生时间、可选评语、平台信息。
4. 如果 Work 的 `end_date` 未设置、且事件时间达到日精度及以上，可同步设置 `end_date`；若过去已有值则不静默覆盖。
5. 将 Work 状态更新、Experience 创建、链接建立、审计记录放入**同一数据库事务**；提交后才分发 Domain Event。
6. 返回更新后的 Work 和 Experience。前端刷新、CLI/MCP 得到一致结果。

**重复点击 / AI 网络重试**：用 `request_id` 或 `idempotency_key`（来自调用者）实现幂等。相同 key 只会创建一次；不同 key 允许确实存在二周目、重新通关等多条历史。禁止 `UNIQUE(subject_id,event_type)` 这种会扼杀真实历史的限制。

### 5.2 不自动生成足迹的情况

- 普通 +/- 进度步进；
- 扫描器发现/索引某个游戏目录；
- 导入已有 `completed` 状态的老数据库；
- 修复数据一致性或重建搜索索引；
- 程序内部重复通知/事件重放。

对于从 `in_progress` 改为 `completed`，UI 应打开简洁的「通关记录」确认表单。为了兼容 CLI，新增专用复合命令；旧 `update_work_status` 只更新状态，避免静默新造时间线事件。后续可在 UI 中提供「只修改状态，不留足迹」选项。

### 5.3 历史 Work 数据兼容

已有 `status=completed` 但没有足迹的作品：
- **绝不批量伪造“完成于导入日”事件**；
- 详情显示一个非阻断提示「已完成，尚无通关足迹 → 补记」；
- 可以只回填用户知道的年份或月份；
- 不强制用户补录所有旧游戏。

## 6. 数据库迁移建议

**先核对真实迁移编号；禁止直接修改旧迁移文件。**

### 6.1 Experience 表（建议独立新迁移）

```sql
CREATE TABLE experience_entries (
  id TEXT PRIMARY KEY,
  subject_type TEXT NOT NULL CHECK(subject_type IN ('entity')),
  subject_id TEXT NOT NULL REFERENCES entities(id) ON DELETE RESTRICT,
  event_type TEXT NOT NULL,
  title TEXT,
  note TEXT,
  occurred_value TEXT,
  occurred_precision TEXT NOT NULL CHECK(occurred_precision IN ('unknown','year','month','day','instant')),
  context_json TEXT NOT NULL DEFAULT '{}',
  visibility TEXT NOT NULL DEFAULT 'normal' CHECK(visibility IN ('normal','archived')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  CHECK ((occurred_precision='unknown' AND occurred_value IS NULL) OR
         (occurred_precision<>'unknown' AND occurred_value IS NOT NULL))
);
CREATE INDEX idx_experience_subject ON experience_entries(subject_type, subject_id, created_at DESC);
CREATE INDEX idx_experience_event ON experience_entries(event_type, created_at DESC);
CREATE INDEX idx_experience_occurred ON experience_entries(occurred_precision, occurred_value);

CREATE TABLE experience_links (
  experience_id TEXT NOT NULL REFERENCES experience_entries(id) ON DELETE CASCADE,
  linked_type TEXT NOT NULL CHECK(linked_type IN ('asset','memory','external_reference')),
  linked_id TEXT NOT NULL,
  relation_type TEXT NOT NULL DEFAULT 'evidence',
  position INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(experience_id, linked_type, linked_id)
);
CREATE INDEX idx_experience_link_target ON experience_links(linked_type, linked_id);

CREATE TABLE experience_request_ids (
  actor_scope TEXT NOT NULL,
  request_id TEXT NOT NULL,
  experience_id TEXT NOT NULL REFERENCES experience_entries(id),
  PRIMARY KEY(actor_scope, request_id)
);
```

注意：多态 `linked_id` 不能用单个传统 FOREIGN KEY 同时指向三个表。必须在 Core + 同事务校验，并在对应对象移除时定义一致清理/失联策略。MVP 可以先仅支持关联 `asset` / `memory`，`external_reference` 预留。

对 `subject_id` 使用外键后，删除 Work 应默认被阻止；让用户选择**归档 Work**，或以显式危险流程删除整个历史（v0.4 不实现这类永久删除）。

### 6.2 仓储扩展

在 Core 新增：

```text
ExperienceService
  create / get / list / update / archive
  link / unlink
  complete_work_and_record_experience [transactional]
```

将仓储 Trait 扩到 `VaultRepository`；不让 `GamesView` 或 MCP 调用数据库直写。复合事务可以定义一个 Repository 专用原子方法，并让 Facade 保持业务编排，不强迫 Facade 自己握持 SQLite Connection。

### 6.3 FTS / 搜索

新增体验标题和简短感想的全文索引，或将体验加入现有 FTS 投影；必须同步维护索引，接受中文分词不足时的 LIKE 回退。**不能为了足迹引入 Elasticsearch 或向量数据库。**

---

## 7. UI/UX：尽可能少操作，不开新大页面

### 7.1 作品详情（GamesView / 其他 Work 详情共用）

```text
┌─────────────────────────────────────────────────┐
│ 血源诅咒             已通关      PC 模拟器       │
│ 封面 / 作品资料 / 评分 / 当前进度                │
│                                                 │
│ [更新进度] [记录经历] [归档作品]                │
│                                                 │
│ 个人足迹                                        │
│  • 完成首次通关   时间未知  [补充日期/感想]     │
│  • ...                                          │
│                                                 │
│ 手记 / 关联图片 / 外部链接 / 本地位置            │
└─────────────────────────────────────────────────┘
```

### 7.2 「记录经历」表单（最多一个小弹窗）

必填：对象（预选当前 Work）、事件类型。  
可选：日期（支持未知/年/月/日）、一句话感想、平台/环境、已有图片链接。  
核心操作：`[保存]`；不应要求上传截图或写长评。

当 Work 状态更新为 Completed 时：先显示「保存通关记录」弹窗，默认事件 `completed`，既可保存事件也可明确选择「只更新状态」。若选择「今天」需展示日期确认。

### 7.3 时间线

- 追加 `experience` 事件类型，显示作品名、事件、发生日期精度、可选缩略图。
- `occurred_precision=unknown` 不得显示为“今天通关”；可在时间线提供“录入时间”视图或“日期待补充”分组。
- 默认突出**有意义的体验**，资产索引变动/每集进度变动不要冲掉真实生活记忆；提供显示过滤。
- 从时间线点击足迹直达对应作品与记录详情。
- 按时间分页，避免全量拉取用户大量资产后在前端排序。

### 7.4 绝不新增五个相似侧边栏

`Games / Works`、`Timeline`、`Assets` 已足够承载体验。必要时仅在已有详情页加入「个人足迹」区块和一个轻量弹窗；以后需求成熟再考虑统一“我的经历”聚合页。

---

# Part B — Reference 资产生命周期系统

## 8. ADR-002：把「有无文件」和「想不想看」分开

现有 `AssetStatus=Active/Missing/Archived` 把可访问性和展示偏好放在一起，不利于断盘和归档的正确判断。

目标概念是 **三个正交维度**：

| 维度 | 推荐值 | 语义 |
|---|---|---|
| **availability（可访问性）** | `available` / `missing` / `source_offline` / `unchecked` | 物理文件能否访问、是否完成验证 |
| **visibility（管理意图）** | `active` / `archived` | 用户是否希望日常看见它 |
| **tracking（索引状态）** | `tracked` / `untracked` | Looma 是否继续扫描和维护索引 |

`untracked` 不能简单删除所有对象，原因是下次扫描可能把它自动导入回来，而且相关足迹不能被连带删除。

### 8.1 面向用户的动作与真实效果

| UI 动作 | 索引记录 | 作品/足迹 | 原始磁盘文件 | 是否容易恢复 |
|---|---|---|---|---|
| **从某合集移除** | 保留 | 保留 | 不变 | 立即重加 |
| **解除作品-资产关联** | 保留 | 保留 | 不变 | 立即重新关联 |
| **归档资产** | 保留，`visibility=archived` | 保留 | 不变 | 随时取消归档 |
| **从 Looma 移除** | 改为 `untracked`（留墓碑/抑制重扫） | 历史保留；关联显示“已移除引用” | 不变 | 显式恢复跟踪 |
| **路径失联** | 保留，`availability=missing` | 保留 | 不变 | 自动/手工重连 |
| **来源离线** | 保留，`availability=source_offline` | 保留 | 不变 | 接回盘恢复 |
| **删除原始文件** | **本阶段不提供** | 不适用 | 可能不可逆 | 不允许误触 |

**归档不是删除；文件丢失不是用户要求删除；不追踪也不等于忘记经历。**

### 8.2 作品状态与归档状态也要分开

已有 `RecordStatus::Archived` 与 `completed/in_progress` 混在同一个字段，不要在这次改造直接移除旧枚举，避免破坏旧数据。

建议新增 `entity_visibility`（或配置层字段），把 `Work.status=completed` 和 `visibility=archived` 独立表示；迁移已有 `Archived` 旧值时，**保留原始值并通过兼容层解释**，待审计与回归测试通过后再做显式转换，不得无声改写。

例子：`Bloodborne` 可以同时是 `completed + archived`；删除安装目录后为 `no_local_install`，足迹不变。

---

## 9. SourceRoot：来源根目录与重连

### 9.1 为什么需要 SourceRoot

绝对路径 `E:\Media\Anime\a.jpg` 无法优雅表达 E 盘改为 G 盘、移动硬盘离线、NAS 暂时不可访问等情况。

SourceRoot 让资产存储成：

```text
root_id = root_media_01
relative_path = Anime/a.jpg
last_seen_absolute_path = E:/Media/Anime/a.jpg
```

Root 的当前绑定可以从 `E:/Media` 改为 `G:/Media`，资产逻辑身份和关系不变。

### 9.2 推荐 SourceRoot 模型

```rust
pub struct SourceRoot {
    pub id: String,
    pub label: String,              // "Media Disk"，用户可读
    pub path: String,               // 绑定路径（本机路径）
    pub source_kind: String,        // local/removable/network
    pub enabled: bool,
    pub state: String,              // online/offline/unverified
    pub last_successful_scan_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub settings_json: Value,       // excludes, symlink policy 等
}
```

`source_roots` 是 Vault 配置实体，不应该绑定某一台设备的盘符为全局永久身份。跨设备备份/恢复时，Root ID 保留，`path` 需要重新绑定。

### 9.3 建议 SQLite 增量模型

避免一上来重建 `assets` 表，可新增：

```sql
CREATE TABLE source_roots (
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

CREATE TABLE asset_lifecycle (
  asset_id TEXT PRIMARY KEY REFERENCES assets(id) ON DELETE CASCADE,
  root_id TEXT REFERENCES source_roots(id),
  relative_path TEXT,
  availability TEXT NOT NULL DEFAULT 'unchecked',
  visibility TEXT NOT NULL DEFAULT 'active',
  tracking TEXT NOT NULL DEFAULT 'tracked',
  last_verified_at TEXT,
  untracked_at TEXT,
  untracked_reason TEXT
);
CREATE INDEX idx_asset_lifecycle_root_rel ON asset_lifecycle(root_id, relative_path);
CREATE INDEX idx_asset_lifecycle_state ON asset_lifecycle(availability, visibility, tracking);

CREATE TABLE asset_scan_suppressions (
  id TEXT PRIMARY KEY,
  root_id TEXT NOT NULL REFERENCES source_roots(id),
  relative_path TEXT NOT NULL,
  reason TEXT,
  created_at TEXT NOT NULL,
  UNIQUE(root_id, relative_path)
);
```

**逻辑删除墓碑**：`asset_scan_suppressions` 防止用户刚点击「从 Looma 移除」，下一次 scanner 又发现同一路径并自动创建。用户主动重新跟踪时移除抑制项。

> 性能/扩展提醒：上面为 MVP 方案。若存在重命名/搬迁和多个 Root 的覆盖，抑制键未来可能需包含文件系统标识或内容指纹；本版先用 Root ID + 规范化相对路径且明示限制。

### 9.4 Root 健康检查规则

扫描前预检：

1. 根目录路径可访问且为目录？
2. 是否具备只读遍历权限？
3. 是否确实遍历成功且无部分错误？
4. 是否满足配置的排除规则？

结果：

- **Root offline / inaccessible** → 该 Root 旗下资产标记 `source_offline` 或维持旧物理可访问性并显示“未验证”；**不允许逐个标 Missing，不允许清理**。
- **Root online but scan partially failed** → 未检出的文件标 `unchecked`；不将未访问路径当删除。
- **Root completely scanned** → 仅此时可以对确证不存在的路径标 `missing`。
- **重绑定根目录** → 按 `root_id + relative_path` 恢复；保持 `asset.id`、原有 Relations、Experience links、Collections 不变。

### 9.5 与现有多路径游戏库的关系

当前游戏可能有多个本地目录 Asset 与同一个 Work 关联；SourceRoot 属于**文件发现/存储层**，不是作品级“安装位置”的替代。变更盘符时应更新目录 Asset 的解析路径，但保持 Work 关系和多个位置关系不丢失。

不能仅依据文件名断定同一 Asset；重连优先用原 Root ID + 相对路径，必要时使用 size、mtime、可选哈希二次确认。完整 SHA-256 不可作为每次扫描必需步骤。

### 9.6 路径与扫描安全约束

- Windows 盘符大小写与分隔符规范化；保留原始路径供 UI 展示；不要一律 `.to_lowercase()` 破坏大小写敏感文件系统。
- 不允许路径解析逃出 Root（`..`、软链接跳出）；默认不跟随符号链接或至少可配置并限制环路。
- 源目录重叠时明确 Root 归属优先级，不得无限重复索引。
- 排除 `.looma` 数据库目录、备份临时目录和应用自身索引输出，防止自我递归。
- 不存在权限的文件报告扫描错误，不能当 Missing。
- 网络盘、可移动盘和受限目录应有超时/取消与异步进度；UI 不可卡死。

---

## 10. 归档、移除和安全清理：API 设计

### 10.1 核心命令

```text
archive_asset(asset_id)
unarchive_asset(asset_id)
unlink_asset_from_subject(asset_id, subject_id)
untrack_asset(asset_id, reason)    // soft untrack, root-scoped suppression
resume_tracking(asset_id)
rebind_source_root(root_id, new_path)
inspect_source_root(root_id)
archive_entity(entity_id)
unarchive_entity(entity_id)
```

方法全部在 LoomaCore 统一授权、验证、调用事务和审计。`untrack_asset` 不删除文件，也不删除足迹。若必须从系统中完全清除记录，留给未来独立的危险操作流程，不混用 `untrack`。

### 10.2 现有 Doctor 的风险隔离

存在 `doctor_cleanup_missing()` 等功能时：
- 保留 inspect/diagnostics，但默认 **dry-run**；
- 不得将 `source_offline`、`unchecked`、`archived`、`untracked` 与 `missing` 混为“可清理项”；
- 「清理失联」不得删除被 Experience/Work/Collection 引用的对象；推荐先归档或软移除；
- GUI 不再把“清理失联”默认设为直接 SQL DELETE；
- CLI/MCP 的 destructive permission 不能绕过上述不变量。

### 10.3 历史依赖和引用策略

一个 Asset 被某条 Experience 引用后，即便用户执行「从 Looma 移除」，足迹仍可打开，只显示 `关联文件已从索引移除 / 原路径未知或不可访问`。不得级联删除 Experience。

如果体验关联需要保留路径文字，可在建立链接时存一份不可用于自动访问的 `display_snapshot`（原文件名/简短来源），避免后续显示空白；快照中不要放入可外传的绝对私有路径。

---

# Part C — 统一架构、MCP、备份与性能

## 11. 核心调用链与事务边界

```text
┌────────────────────────────────────────────────────────────┐
│ React/Tauri        CLI                MCP                  │
└──────────────┬─────┬──────────────────┬────────────────────┘
               └─────┴──────────────────┘
                            │
                  LoomaCore (Facade)
             validation / actor / permission
                            │
          ┌─────────────────┼─────────────────┐
          ▼                 ▼                 ▼
  ExperienceService   AssetLifecycle    WorkService
          └─────────────────┼─────────────────┘
                            ▼
                  Repository Transaction
                            │
                SQLite + indexed references
                            │
                     commit succeeds
                            │
                 event emit / UI refresh
```

**事务原子性是验收硬约束**：`Work.Completed` + `Experience.Created` + `Audit` 在同一事务提交/回滚。领域事件在提交后发出，事务提交失败时不发布事件。不允许“状态已完成，但创建足迹失败还返回成功”。

如果跨进程 CLI/MCP 与桌面同时访问同一个 Vault，现有 `Arc<Mutex<Connection>>` 只保护**单进程共享连接**。必须实测多进程 SQLite WAL 锁与 busy timeout，特别是扫描、迁移、恢复操作。必要时对高风险操作提供独立 Vault 写者锁/单写者协调；禁止假装 `Arc<Mutex<...>>` 能解决进程间并发。

## 12. MCP 外部 AI 接口

### 12.1 工具（建议新增）

| MCP tool | 用途 | 最小权限 |
|---|---|---|
| `list_experiences` | 按 Work、时间、类型查询 | ReadOnly |
| `get_experience` | 获取单条足迹 | ReadOnly |
| `record_experience` | 新增个人经历及可选感想 | ContentWrite |
| `update_experience` | 修改日期/感想等个人内容 | ContentWrite |
| `complete_work_with_experience` | 原子完成作品并写足迹 | ContentWrite |
| `archive_experience` | 隐藏但保留记录 | MetadataWrite |
| `list_source_roots` | 列出来源根目录（限制绝对路径暴露） | ReadOnly |
| `get_asset_lifecycle` | 资产可用性/显示状态 | ReadOnly |
| `archive_asset` / `unarchive_asset` | 控制显示，不触碰文件 | MetadataWrite |
| `unlink_asset` | 解除单个作品关联 | MetadataWrite |
| `untrack_asset` | 从库停止追踪；带抑制规则 | 默认 **不开放 MCP**，未来 FullDestructive + 显式确认 |
| `rebind_source_root` | 修改 Root 扫描范围 | 默认 CLI/桌面专用；未来显式授权 |
| `delete_original_file` | 删除物理文件 | **不实现** |

保留旧 MCP 工具兼容性；新工具只通过 Facade 操作。对 `record_experience` 需要识别 `subject_id` 而不是根据模糊标题猜写；让 Agent 先查找或用户明确选择对象。

### 12.2 权限策略

**安全默认**：未配置的 MCP 客户端应为 `ReadOnly`。若现有实现默认 `ContentWrite`，在这一阶段整改，并同步 README 的配置例子、用户迁移说明和测试。让用户明确授权 ContentWrite 给 DSH / Antigravity 后，才允许它更新个人记录。

`MetadataWrite` 允许低风险结构和归档操作，`ContentWrite` 才允许新增/修改个人经历和自由文本；`FullDestructive` 不自动意味着允许原始文件删除（本阶段没有这种 API）。

如果 `vault_doctor` 有 cleanup、`delete_asset` 等老工具：必须继续由更严格的检查拦截，**不可通过别名绕过权限**。

### 12.3 示例 MCP 请求语义

```json
{
  "tool": "record_experience",
  "arguments": {
    "subject_id": "<existing-work-id>",
    "event_type": "completed",
    "occurred_precision": "unknown",
    "occurred_value": null,
    "title": "完成首次通关",
    "note": null,
    "context": {"platform": "PC", "environment": "emulator"},
    "request_id": "client-generated-unique-id"
  }
}
```

MCP 对外响应必须有机器可读错误码：`not_found`、`invalid_time`、`permission_denied`、`duplicate_request`（也可返回原结果）、`conflict`、`source_offline`。不要让 AI 靠解析错误字符串判断。

### 12.4 MCP Resources

可新增 `looma://work/{id}/experiences` 和 `looma://experience/{id}` 的 read-only 资源视图；Resources 不必重复每个分页查询工具，保证现有 `looma://work/{id}` Resource 的历史结构不被静默破坏。

## 13. CLI 命令

```bash
# 查询作品足迹
looma experience list --work <id>

# 补记：未知日期也能保存
looma experience add --work <id> --event completed --date unknown --platform PC --environment emulator

# 带具体日期完成（命令语法由 CLI 现有库适配）
looma work complete <id> --date 2026-11-14 --note "终于通关"

# 来源根目录
looma source list
looma source inspect <root-id>
looma source rebind <root-id> "G:/Media"

# 安全归档与索引移除
looma asset archive <id>
looma asset untrack <id> --keep-original --confirm
```

CLI 输出必须区分：`archive`、`untrack`、`delete original`；最后一种不能存在于 v0.4。所有新增命令都要有 `--help`、错误码和可脚本化 JSON 输出模式（如果项目已有全局 JSON 模式则沿用）。

## 14. 备份与恢复：个人记忆比索引重要

新增体验和 SourceRoot 后，Portable Backup 必须一起升级；不能出现“桌面显示有通关记录，但导出后丢了”。

建议快照：

```text
backup/
  manifest.json
  vault_dump.json
  experiences.json
  source_roots.json
  memories/*.md
  ...existing datasets...
```

`experiences.json` 可以是 `vault_dump.json` 中的权威子集或独立权威文件，**必须明确唯一权威来源**，不可两份恢复时互相覆盖。建议既保留兼容旧 `vault_dump.json`，又允许新版本通过明确版本 Schema 扩展；若单独存储 `experiences.json`，manifest 中标注它是权威、dump 中不重复（或只作派生）。

必须覆盖：
- Experience / links / idempotency metadata 按规定备份；
- 生命周期、Root ID、relative_path、用户归档选择、抑制规则备份；
- 本机特有 root.path 可备份为提示字段，但恢复到其他电脑必须默认标为 `unverified` 并提供重新绑定；
- 不备份大型原始媒体，默认不导出可直接访问的私有绝对路径到第三方服务；如果用户选择明文导出，应解释路径泄露风险；
- manifest 升级 schema version、文件清单、sha256，恢复前验证；
- 旧备份可恢复，新备份对旧程序不保证兼容，但须给出清楚的版本错误；
- 备份快照一致性：SQLite WAL 模式应使用 SQLite Backup API / 一致事务快照，不直接裸复制活动中的 vault.db；
- restore 需先在隔离临时库验证，再按计划原子切换或事务导入，失败不覆盖正常 Vault；
- 至少做一次**全量导出 → 新 Vault 恢复 → ID/关联/足迹比较**的自动测试。

## 15. Search / Timeline / 高性能

### 15.1 搜索

- 体验的 Work title、event type、note、occurred date 可查询；
- 搜索结果保留对象类型 `asset/work/experience/memory`；
- 资产归档默认不在普通结果显示，提供「包括归档」开关；
- 用户主动 `untracked` 的资产不当作正常可用文件展示，但历史关系可通过足迹查询到占位信息。

### 15.2 时间线

- `experience` 是独立类型，可按“有意义的足迹”过滤；
- 不自动把每一次 1 集/1 话增量进度写成足迹；
- 不把 `created_at` 伪装为真实完成时间；
- 时间线按参数限制和分页，新增索引，不全量先读到 React；
- 扫描器事件/数据库 audit 不自动进入人生时间线。

### 15.3 性能底线

- 全盘扫描增量 + 按需哈希，禁止为了归档/足迹触发重复扫描；
- Root 离线判断使用一次根目录预检，不逐个文件阻塞 UI；
- 文件系统与批量扫描运行后台任务，支持取消和进度；
- 列表必须分页，关联项目按批量查询解决 N+1；
- 至少提供 1 万/10 万资产规模的可复现基准脚本（只需测试，不要求夸大性能数字）。

---

# Part D — 开发计划、验收与风险控制

## 16. 建议分五个小阶段提交

### Phase 0 — 基线与保护（先完成）

**改动目标**：零功能变化，锁定现状。

- 记录 Git HEAD、现有 Migration 与 API 列表，保留 `git status`；
- 运行并记录 `cargo test --workspace`、前端构建/TS 类型检查、现有 smoke；
- 用复制的测试 Vault 生成并验证一份备份；
- 验证 MCP 默认权限与 CLI/desktop 同库并发行为；
- 画清 Work/Entity/Asset/Relation/Memory/Timeline/Backup 现有调用链。

**完成标准**：测试与风险报告可复现，能回到基线版本。不要为了使测试绿而降低断言或删除测试。

### Phase 1 — 足迹基础（最高优先）

- 新 Migration: `experience_entries` + 索引 + links + 请求幂等；
- Model、Repository、LoomaCore API；
- 原子 `complete_work_and_record_experience`；
- 精度日期验证、重复记录/二周目支持；
- `Experience` 加入时间线和备份导出/恢复；
- Core 单元与集成测试。

**验收情景**：一款已经存在的游戏，即使没有本地文件也能补录首次通关；失败不出现半成功。

### Phase 2 — 用户端与 AI 端闭环

- GamesView 增加「记录经历」「完成并留足迹」操作；
- 尽可能复用其他作品详情组件（如果现在组件不共享，先提取小共用组件而非重构所有视图）；
- TimeLine 的 `experience` 类型；
- CLI + MCP Tool、Resource、权限限制与文档；
- 让没有日期和截图的通关记录可以正常显示。

**验收情景**：在 DSH / Antigravity 明确授予 ContentWrite 的前提下，AI 可查询已有作品 ID 并添加通关足迹，GUI 时间线刷新后可见，备份中可恢复。

### Phase 3 — 来源与安全生命周期

- 新 Migration: SourceRoot / AssetLifecycle / Suppression；
- 从旧绝对路径增量绑定 Root（不得偷偷改既有 asset.id）；
- 来源离线、扫描失败、完整扫描成功的状态机；
- Root Rebind，确认引用、作品本地目录与封面关系不会断；
- Archive / Unarchive / Unlink / Untrack / Resume；
- Doctor 的只读默认与危险操作隔离；
- UI 资产详情最小按钮、来源列表状态。

**验收情景**：拔掉 F 盘后所有资源显示来源离线，接回或改 G 盘后恢复；不会产生新的重复资产、不会自动清理旧记录。

### Phase 4 — 稳定化与数据安全

- 备份 Manifest 和 Schema 版本支持新数据集；
- 恢复、重扫、跨盘重绑、MCP 权限、多进程并发集成测试；
- 10 万文件模拟数据基准；
- 兼容旧 Work.status、旧 Asset.status、旧 Vault；
- README 的“归档 vs 移除”语义、手工补记指南、MCP 权限配置、已知局限更新；
- 输出完整验收证据，不允许仅凭 crate 存在就标 PASS。

**验收情景**：旧数据可以无损升级，跨备份恢复后经历与索引引用仍稳定。

## 17. 最小验收测试矩阵（全部必须通过）

| 编号 | 测试输入/动作 | 预期结果 |
|---|---|---|
| T01 | 已通关《血源诅咒》，未知日期，没截图 | 能生成 `completed` 体验；日期显示未知 |
| T02 | 作品 Completed，但已有历史无体验 | 不回填虚构的事件；允许用户补录 |
| T03 | 相同 request_id 重试两次 | 只创建一条足迹，结果可幂等返回 |
| T04 | 二周目再次完成，新的 request_id | 允许两条不同完成/重玩经历 |
| T05 | Work 状态更新成功但 Experience 插入失败 | 整体事务回滚；审计不能假报完成 |
| T06 | 日期输入 `2026-02-30` / 精度不匹配 | 被验证拒绝，用户看到明确错误 |
| T07 | Experience 关联截图后将截图索引移除 | 体验保留；引用显示已移除、不打开旧路径 |
| T08 | 游戏目录卸载 | Work/Experience 保留，位置显示失联 |
| T09 | 源盘拔掉或网络盘不可达 | Root offline；无批量 Missing/DELETE |
| T10 | 扫描中一部分目录权限不足 | 未检出文件保持 unchecked，不能清理 |
| T11 | SourceRoot E: 改绑定 G: | 资产 ID、Relations、Work 多位置绑定不变 |
| T12 | 用户主动 Untrack 后再次扫描 | 不会自动重新导入（suppression 生效） |
| T13 | Resume Tracking | 抑制消除，同一记录恢复跟踪 |
| T14 | 归档资产或作品 | 默认视图隐藏，搜索包括归档时可找到，原文件不变 |
| T15 | `MCP ReadOnly` 调用 `record_experience` | 明确 permission denied；无副作用 |
| T16 | `MCP ContentWrite` 新建足迹 | 成功、审计正确、GUI/CLI 可查到 |
| T17 | MCP / Doctor 试图删除原文件 | 无此 API；不能通过别名绕过 |
| T18 | 导出 → 新 Vault 恢复 | Work/Experience/links/Roots/归档与抑制均保留 |
| T19 | 旧 Vault 直接迁移 | 可启动、旧数据与 ID 不变；没有伪造足迹 |
| T20 | 两进程同时读写 Vault | 明确支持的操作成功或给出可恢复 Busy/Lock 错误，无数据库损坏 |
| T21 | 审计表插入失败 | 复合业务写入不出现“已成功却无审计”的错误状态 |
| T22 | 100k 资产分页 + 查询 | UI 不全量加载、不阻塞；记录测得指标 |

额外建议：恢复前文件哈希损坏、Root 嵌套、路径 Unicode/大小写、符号链接/循环、移动目录、重复游戏候选、同名文件不同内容，作为边界回归用例。

## 18. 变更文件范围（候选，Agent 自行核对实际结构）

```text
crates/core/src/models/experience.rs        [new]
crates/core/src/models/source_root.rs       [new]
crates/core/src/models/asset_lifecycle.rs  [new]
crates/core/src/models/mod.rs              [edit]
crates/core/src/services/mod.rs            [edit]
crates/core/src/facade.rs                  [edit]
crates/core/src/events.rs                  [edit]
crates/database/src/migrations/00N_*.sql   [new only]
crates/database/src/migration.rs           [edit]
crates/database/src/db.rs                  [edit or split focused modules]
crates/mcp/src/server.rs                   [edit]
crates/cli/src/main.rs                    [edit]
apps/desktop/src/components/GamesView.tsx [edit]
apps/desktop/src/components/Timeline*.tsx [locate actual file]
apps/desktop/src/types*                   [edit]
apps/desktop/src/components/Experience*   [new reusable small component]
docs/                                      [add ADR + tests / usage]
```

禁止为了这项功能把整个数据库层或 React UI 重写。对于已有 `db.rs` 超过两千行的情况，允许**仅**把新增 Experience / SourceRoot Repository 提取成独立模块，同时用最小接口接入。

## 19. 关键架构不变量（必须写进测试）

1. `Experience != Work.status != Memory != DomainEvent != AuditLog`。
2. Experience 不依赖文件存在，也不因 Asset untrack 而级联删除。
3. UI、CLI、MCP 调用统一 LoomaCore 业务 API。
4. “完成 + 足迹 + 审计”是单事务，事务后才能发事件。
5. Source Offline/扫描失败不导致 Missing 清理或资产删除。
6. Archive、Unlink、Untrack 与物理文件删除是不同操作。
7. 同一 Work 可以拥有多条真实体验历史；自动导入不会制造历史。
8. 备份与恢复必须保持对象 ID、关联、时间精度、抑制规则。
9. 默认 Reference Mode；不挪、不复制、不上传原始媒体。
10. MCP 默认只读，写入需要明确授权；不存在无权限的原文件删除途径。
11. 所有新数据遵守可导出/可迁移，不把个人记忆藏在无法恢复的 UI 状态中。
12. 增量扫描不能修改原盘的用户文件内容。

## 20. 性能与可扩展性的工程取舍

### 20.1 不为“未来可能”提前造复杂系统

本阶段不做：专用事件流数据库、向量搜索、复杂分布式锁、多端实时冲突解决、插件沙箱、自动录屏、AI 分类中心、完整媒体托管。

### 20.2 可扩展方式

- 新体验类型 → `event_type` + UI 模板，不修改大量 Core 分支；
- 新对象领域 → 先复用 Entity/Work，确有复杂行为再建专门 Domain；
- 新资产类型 → 扫描器/Metadata Provider 扩展，不重建 Asset；
- AI → MCP/CLI，而不是在 Looma 内部署大模型；
- 云端 → Backup 包 + CloudStorage Provider，明确只传结构化数据与小附件，非实时同步。

### 20.3 版本策略

该专项建议称为 **Looma v0.4（设计里程碑）**，但**不得据此擅自修改现有 Cargo/package 版本**；实际版本与发布节奏由用户决定。

---

## 21. 交给反重力的逐阶段任务指令（可直接复制）

> 你现在是 Looma 的实施工程师。请把 `Looma_v0.4_Experience_and_Reference_Lifecycle_Design.md` 作为本轮专项开发基线，同时对照当前本地仓库及现有架构。先执行 Phase 0 基线保护、检查现有 Migration、Core API、MCP 权限和测试，再制定 4 个可独立验收的开发 PR/Commit 计划。不要直接一次实现所有功能。第一阶段请优先实现 ExperienceEntry、完成作品原子工作流、时间精度与备份兼容；第二阶段接入作品详情、时间线、CLI、MCP；第三阶段再实现 SourceRoot 和 Reference 生命周期；最后做集成测试与文档。所有改动必须遵守 Reference Mode 原文件只读、禁止自动删除、历史足迹不因文件丢失而消失、MCP 默认只读、迁移只增不改。每阶段提交代码 diff 摘要、测试输出、验收样例和风险清单。不要用“全部完成”代替真实测试结果。

---

## 22. 产品最终验收故事

用户打开 Looma，找到《血源诅咒》，点「记录经历 → 完成首次通关」。他不知道日期，没保存截图，填写“PC 模拟器”，不写评语。Looma 保存这条事件，并在该游戏和个人时间线中显示“完成首次通关 · 时间待补充”。几个月后他删掉游戏文件或卸载模拟器，作品仍然可以被搜索，通关足迹不消失；之后可以补上日期或感想。

另一边，用户的 F 盘里有大量模型、截图和素材。Looma 扫描并引用它们，不搬移。F 盘拔掉时只提示“来源离线”；插回或改为 G 盘时，通过 Root 重绑定恢复同一资产身份。用户不想再看某些东西时归档，想解除索引时选择“从 Looma 移除，保留原文件”。再次扫描也不会强行导入这些已移除文件。全部结构化数据可备份和恢复，DSH/Antigravity 可在用户授权下通过 MCP 补记和查找足迹。

> **真正的完成标准不是页面上多了多少按钮，而是：文件越来越乱也不影响寻找；原文件不在了，个人经历仍然能留下。**
