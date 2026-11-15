# Looma v0.6｜跨设备游玩事实与可选 AI 识别：最终架构裁决及实施契约

> 文档性质：最终技术决议 / 交付 Antigravity 执行（不是继续无限讨论的 RFC）  
> 日期：2026-11-15  
> 讨论依据：Antigravity《Looma v0.6 架构交叉讨论回复》（基准 `75eb31a2529590d204fc2f64ba56f973f612eeed`）  
> 决议：三项方案 A 原则获批准，但不得照抄原三表 DDL；实施前修订 `005`，明确导入批次、未匹配记录、来源身份、历史快照与删除语义。  
> **执行方式**：按有依赖的独立 PR 实施；每个 PR 本地完成测试、报告、提交后可以继续。仅当触发“停止条件”时回报用户决策。

---

## 0. 这次要解决什么，不解决什么

Looma 的目标不是仿制 Steam，也不是成为 Switch 常驻后台工具，更不是内置一个大型 Agent。它应该在**不碰原始游戏和存档**的前提下，保留可解释的跨平台使用事实，并把值得纪念的通关、二周目、全收集变成个人足迹。

三个领域必须分离：

- **Work / Entity**：作品身份与当前个人状态，包括游戏名称、别名、当前进度、评分。
- **Activity Fact**：来源系统报告的游玩统计。统计具有 `source + account/profile + external_game_id + captured_at` 的来源语义，可刷新、可矛盾、可被修正；不等于通关。
- **ExperienceEntry**：用户确认或主动写入的个人经历，允许时间未知、重复通关与独立感想，且不因源数据移除而消失。

### v0.6 核心 MVP

实现 **Switch NX Activity Log 离线 JSON → 选择 Profile → 预览并映射 Work → 原子导入活动统计 → 来源分栏展示 → 用户主动标记通关 Experience**。

### v0.6 后续迭代（不得阻挡核心 MVP）

- Steam 官方只读活动/成就 PoC。
- Windows 进程观察器 PoC，不作为官方游玩时长。
- 可选 AI 识别适配器 PoC：单轮文本 JSON + 待审核建议。无需 Tool Calling。

暂缓：Switch 常驻 sysmodule、局域网实时监听、远程 HTTP 暴露、跨源总时长默认相加、抓取破解游戏成就文件格式、内置 LLM Agent、自动根据时长宣告通关、强制上传用户文件。

---

## 1. 最终裁决

| 决策 | 结果 | 必要约束 |
|---|---|---|
| 1. 持久化 | 采用独立活动事实层 | `005` 必须是新增迁移；不得修改 001–004；必须支持未匹配 Work。 |
| 2. MVP | 只打通 Switch JSON | 正式 UI 不依赖 Switch 在线、不碰 SD 卡系统数据；Steam、PC 观察器分期。 |
| 3. AI | 使用只读单轮建议 | 无 Tool Calling；模型输出一律不可信；本地校验后进入 Review Queue；由用户确认、Core 执行。 |

**额外裁决：原报告“三张表刚好够”不成立。** 推荐最低 **四张表**：`activity_sources`、`external_game_identities`、`activity_import_batches`、`playtime_observations`。第四张导入批次表不是炫技扩展，而是确保重复导入、审计、时间回溯和故障定位的必要机制。

---

## 2. 关键的数据建模纠正

### 2.1 来源与用户 Profile

`activity_sources` 代表**一个可区分的统计来源实例**，如「这台 Switch 的 Profile A」，而不是笼统的 `switch`：

- `id`：Looma 自己的稳定 ID。
- `provider_kind`：`switch_nx_activity_log` / 未来 `steam_web_api` / `windows_observer`。
- `device_id`：Looma 注册的设备身份，独立于展示名称；可由用户自行创建，不依赖设备序列号。
- `profile_key`：来源的 Profile 身份（需要脱敏展示，限制日志输出）；不要以用户昵称作为唯一键。
- `label`、`enabled`、`created_at`、`last_imported_at`、`metadata_json`。
- 唯一性要按 `(provider_kind, device_id, profile_key)` 约束，不能仅以 `provider_kind` 约束。

**重要**：导出包含多个 Profile；预览时可暂时解析必要信息供用户选择，但未勾选 Profile 的活动不得写入 Vault。不能声称未经解析就能知道 Profile 列表。

### 2.2 外部游戏标识与作品映射

`external_game_identities` 表示 `provider + external_game_id` 的**作品候选身份**，不是一次游玩，也不是个人 Profile。应包含：

- `provider_kind`、规范化 `external_game_id`（Switch Title ID 应校验 16 位十六进制，不应假设大小写有区别）。
- `work_id NULLABLE`：允许导入了游戏但尚未匹配到任何 Work；不得为了保存统计强迫自动建立一个 Work。
- `title_hint`、`mapping_status` (`unmatched` / `suggested` / `confirmed` / `ignored`)、`verified_by_user`、`created_at` / `updated_at`。
- 对 `(provider_kind, external_game_id)` 建立唯一约束；映射到现有 Work 必须优先用已确认映射，再考虑别名/标题候选，由用户选择。
- 同名不同 Title ID 可以映射到同一个 Work，但**不允许仅凭名称自动合并**。DLC、试玩版、合集、不同版本等必须有预览和拒绝选项。
- 删除 Work 时，优先 `ON DELETE SET NULL` 或阻止删除；不能无提示级联销毁历史游玩事实。

**不要**在 `playtime_observations` 中强制 `work_id NOT NULL`。统计按 `source_id + identity_id` 保存，Work 映射可以以后再补，历史快照仍能显示在“待认领游戏”中。

### 2.3 导入批次

必须有 `activity_import_batches`：

- `id`、`source_id`、`file_sha256`、`exported_at`、`imported_at`、`export_version`、`accepted_count`、`rejected_count`、`warning_count`、`status`。
- 以 `(source_id, file_sha256)` 识别完全重复的已完成导入。**仅在事务成功后**批次才算 committed；失败不得留下假成功批次。
- 原始文件可以默认**不复制进 Vault**；仅保存必要的格式版本、摘要、哈希、来源与统计。除非用户明确选择保留原始导出，否则不永久保存整个 JSON，尤其不能自动持久化未选 Profile 的数据。
- 重新导入同一文件但选择**不同 Profile**应是合法操作；幂等性必须以具体来源实例为作用域。

### 2.4 历史游玩快照

用 `playtime_observations` 保存每次来源报告的事实：

- `id`、`import_batch_id`、`source_id`、`identity_id`。
- `cumulative_seconds` (INTEGER CHECK >= 0)、`launch_count` (可空)、`first_played_at` / `last_played_at` (可空)、`captured_at`、`data_quality` 与可选受限大小的 `summary_json`。
- `UNIQUE(import_batch_id, identity_id)`；按 `(source_id, identity_id, captured_at DESC, id DESC)` 建立查询索引。
- 默认保留历史快照；“最新展示”通过查询 latest observation 得到，而不是覆盖唯一一条记录。
- 导入累积统计不做 `+=`；不能根据两个累计数字直接算出无误的“新增游戏时间”。
- 最新值小于历史值时标记 `baseline_reset_or_anomaly`，保留旧快照；UI 显示“新报告 2h / 曾报告 85h，可能重置”，**绝不自动合并成 87h，也不静默覆盖**。
- 旧导出晚于新导出才被导入时，不得仅按 `imported_at` 抢占最新观测：明确以来源的 `exported_at` / `captured_at` 排序，时间异常时给出警告。

### 2.5 迁移与一致性

请将以上作为设计约束，由 Antigravity 输出实际完整的 `005_activity_and_external_identity.sql` 并实现对应 repository；并非要求逐字照搬本 RFC 的列名。

- 必须实际开启 `PRAGMA foreign_keys = ON` 并验证 FK 生效。
- `activity_sources` 的普通禁用/解绑不能意外级联删除旧快照。明确区分停用来源、解除映射、删除活动数据。
- Snapshot 与 Mapping 的保存、Import Batch、审计必须位于同一事务；Core 对外提供 `preview_*` 和 `commit_*`；绝不暴露“模型直接写表”的入口。
- **必须扩展备份与恢复**（包括 Manifest 版本、表内容、SHA 校验和、恢复事务及新旧版本兼容）。没有备份恢复覆盖时不得发布 005。
- 严格保留旧库升级、重复导入、断电/约束故障回滚测试。不得修改 001–004。

---

## 3. Switch MVP 交互契约

```text
点击「导入 Switch 游玩记录」
    ↓
选择 NX Activity Log JSON（大小和版本限制）
    ↓
Preview：校验 JSON + 显示 Profile + 选择需要导入的用户
    ↓
为每个 Title ID 展示：来源游戏名 / 时长 / 作品匹配候选
    ↓
用户选择：关联已有作品 / 新建作品 / 暂不匹配 / 忽略
    ↓
显示重复导入、时间倒退、统计重置等警告
    ↓
Commit：确认输入文件未发生变化 → 单事务提交
    ↓
游戏详情：Switch（Profile A）统计卡片 + 来源记录时间
    ↓
【可选】用户主动点击「标记通关」生成 Experience
```

### 解析原则

- NX Activity Log 文档提供 `exportTimestamp`、`exportVersion`、`users[].id/name/titles[]`、`title.id/name/summary/events`；`summary.playtime` **单位为秒**，不能误当毫秒或小时。
- `firstPlayed` / `lastPlayed` 为 0 时表达“未记录”；不能显示为 1970 年。
- 解析未知字段应容忍，但未知/不支持的 **exportVersion** 要明确兼容策略；不能默默假定结构相同。
- `events` 可以缺失/为空；MVP 以 `summary` 为主，不因没有 `events` 导入失败。以后如保存 session 事实再单独设计。
- 仅导入必要字段；原始记录属于外部来源自报，不保证绝对正确。日期异常与时间变更需要如实标注。
- 大文件采用明确的大小上限及用户提示，避免 JSON 炸弹或极端数组占满内存；所有导入工作应在异步任务中完成，UI 有取消/失败提示。
- Preview/Commit 之间必须有文件内容哈希校验，防止预览的是文件 A，提交时实际读取被替换的文件 B；用户选择必须指向已验证的 Source/Profile/Title ID，而不只是候选列表下标。

### Work 匹配与来源展示

- 来源的游戏名不是 Looma 权威作品名；已确认的 Work 名称不能被下一次导入的游戏名覆盖。
- 已有映射直接复用；模糊候选只作建议，用户可拒绝；外部 ID 缺失或错误应进入“待处理”，不虚构 ID。
- 列表可显示“Switch：约 N 小时（Profile A，来自 YYYY-MM-DD 导出）”，但不得将来源统计自动标记为 Completed。
- 同一游戏有多台 Switch、不同 Profile 的统计时，分来源显示；跨源默认不合并时长。

### Experience 的正确幂等规则

- **导入游戏活动事实 ≠ 用户通关**。导入本身不得自动产生 `completed` Experience。
- 用户点击“标记通关”时，产生新的操作请求 ID；**同一提交的网络/UI 重试复用请求 ID**，避免双写。
- 用户第二次实际通关时，生成新的请求 ID，可产生第二条 Experience。
- 绝不能把 `switch_import_<user_id>_<title_id>` 作为唯一永久的通关幂等键，这会误杀二周目。
- 时间默认 unknown；不根据导出日期、最后游玩日伪造通关时间。上下文可记录平台 Switch、Profile 友好名称、来源数据 ID（不要暴露原始私密 ID）。

---

## 4. 多平台时长与成就的后续契约

### 4.1 时长不等于时长总和

同一游戏可能有 `Switch: 40h`、`Steam: 60h`、`Windows 观察器: 20h`。Windows 与 Steam 的统计会重叠；UI 默认只分来源展示，**不得默认合计为 120h**。未来“总计”必须明确来源组合及去重/不去重风险。

### 4.2 PC 本地观察器只是估算值

- 进程监控需要具体 EXE **绝对路径或稳定资产绑定**，不能仅凭 `game.exe` 命中所有游戏。
- 模拟器主进程不是一款游戏：除非能可靠读取 ROM/Title ID 或用户显式指定会话，否则不得归到某款游戏。
- 简单“每 15 秒进程在就 +15 秒”虽比启动/退出差值好，但仍有启停边界误差、前后台挂机、系统休眠和应用崩溃问题。PoC 需要基于 **单调时钟、观察间隔上限、系统睡眠恢复检测与可恢复的会话断点**，并明确“估算时长”。
- 观察器默认关闭；用户明确授权才能运行、记录窗口/进程。不得扫描不相关的个人应用。

### 4.3 Steam 与成就

- Steam 后续支持只读；遵循实际 API 权限与隐私条件。密钥使用操作系统凭据存储，**不得写入 Vault 备份、日志、诊断包或 AI 提示词**。
- 区分 **平台报告的解锁事实**、**用户自定义打卡**、**个人 ExperienceEntry**；从成就/累计时长不能反推用户已通关。
- 第一版不为不同模拟器/非官方版本强行兼容碎片化成就文件。若未来扩展，使用用户明确选择的只读本地适配器。

---

## 5. AI 分析契约：接口开放，但不让模型成为数据库操作者

AI 功能建议安排在 Switch MVP 稳定后的独立 PR。它不要求任何 Tool Response，也不与 MCP 核心调用路径耦合。

```text
用户授权 → Looma 选择最小必要元数据 → 可替换 AnalysisProvider
→ 单轮请求（普通文本 / 可选支持的 JSON Schema 模式）
→ 本地严格 JSON 结构校验、字段白名单、长度和数据范围检查
→ Review Queue 待确认 → 用户选择确认 → LoomaCore 写入/审计
```

- 第一版只做游戏目录名、EXE 名称等**文本识别**；图片/正文/隐私文件另行授权。
- 兼容 OpenAI-compatible 或自定义代理，但先经 **Capability Probe** 验证：普通对话可用、能否返回有效 JSON、能否拒绝危险内容、超时和限流处理。不要假设 Gemini 网页反代支持 JSON Schema 参数；Prompt 要求 JSON 并不等于接口保证 JSON。
- 解析失败、拒绝、网络故障都不得创建/修改资产；可本地重试 0–1 次，之后将失败原因反馈用户。明确最大输出长度，防止不可信大响应。
- 模型不得自行分配 Work ID、也不得通过输出文本触发文件读写命令；本地匹配及主键分配只能经 Core。
- 默认关闭联网分析；展示将发送的数据摘要，不发送完整绝对路径、API Key、环境变量、私人文件正文和 SD 卡 Profile 原始 ID。
- 待审核建议需标注来源模型、生成时间、输入指纹、推断依据及 `pending/accepted/rejected`；已由用户确认的作品信息不能被后续模型覆盖。
- 不强制为所有猜测建立持久队列表：在设计 AI PR 时评估是否先用临时 Review 状态与现有 `Suggestion` 基础设施；如确需跨会话保留，再增加新迁移及备份契约。

---

## 6. v0.5.1 必须独立修复的四项

不将它们塞进活动导入的 PR。但**正式发布 v0.6 前必须全部通过**。

1. Entity 更新的陈旧数据覆盖：使用在 SQLite 内 **原子 compare-and-swap** 的条件更新，校验 `affected_rows == 1`；提交旧版本时明确冲突，不盲写。`updated_at` 若无法保证每次写入唯一且单调，应引入明确的整数 revision 并相应顺延迁移编号；只在 Rust 内先比较后更新不算 CAS。
2. 缩略图安全：解码前检查文件大小/图片尺寸/像素总数，并发上限放在后端（前端限制不是安全边界），损坏图和极端图返回错误占位；限制缓存大小并验证不会触碰原图。
3. 统一 IPC 错误呈现：`AssetsView` 等子组件不能把失败当 `null`/空列表；批量操作必须呈现真实部分成功，提供可重试状态。
4. 资产分页：至少 200 个资产的隔离测试库，确认前 50 个之外可通过列表翻页正常浏览、筛选、排序，搜索结果不受当前页限制。

若 v0.5.1 引入新的 DB Migration，**按真实提交顺序分配版本号**，不得提前把 `005` 同时用于两个不同迁移；可将活动迁移顺延为 `006`。不能改写已发布的历史迁移。

---

## 7. 推荐独立 PR 与验收门槛

### PR S0 — v0.5.1 Stable Baseline（独立 hotfix）

- 上述四项缺陷修复，恢复数据准确性、列表可浏览性。
- 测试：两个外部进程/连接陈旧更新冲突、巨大图并发、损坏图片、IPC 错误渲染、200 资产全列表分页。
- 测试使用临时 Vault 和临时文件，不碰真实资产。

### PR A0 — Activity Contract & Migration

- 先编写可迁移的表结构、Repository 接口、备份恢复兼容、数据模型及测试。
- 验收：旧 Vault 升级、全新 Vault、FK、未映射 Title、Profile A/B 隔离、重置快照历史、Work 删除不级联丢活动、导入故障回滚、备份恢复。

### PR A1 — Switch JSON Parser & Preview

- 不写数据库的纯解析服务，支持多用户、摘要无 events、时间为 0、语言名称、无效 Title ID、坏 JSON、大小上限、格式版本、文件哈希。
- Preview 选择 Profile、确认标题映射或保留 unmatched、识别重复批次和计时回退。
- 使用来自真实导出格式的**脱敏夹具**，不能仅用手写理想化 JSON。

### PR A2 — Import Commit & UI

- Core 事务写入来源/Identity/Batch/Observations/审计；必要时同事务建立新 Work；UI 导入与详情分栏。
- 验收：同一文件导入两次无重复；两 Profile 同一 Title ID 时长互不覆盖；二次新导出产生新历史快照；新累计变小不误加；中途注入错误全部回滚。

### PR A3 — Experience & Cross-Platform Integration

- 用户主动通关：即使没有截图、没有明确日期也能录入；重试不重复、二周目不阻止；Work 与个人时间线同步展示。
- 来源统计被删除/解绑后 Experience 仍在；设备离线时作品仍在。

### PR B0 — Steam / Windows Playtime PoC（后续独立可选）

- 最先确定真实数据来源、重叠统计与隐私。避免在核心 MVP 未可靠时实现常驻进程守护。

### PR C0 — AI Game Identification PoC（后续独立可选）

- 仅单轮文本和 JSON 校验、待审核建议、隐私预览、失败降级；不依赖 tools、不过度设计 Provider SDK。

**每 PR 必须交付**：提交 SHA、改动路径、修复前失败用例（若为缺陷）、自动化测试、实际 UI 操作、数据安全说明、遗留风险清单。完成后可以继续下一 PR，不需要重新进行宏观架构讨论。

---

## 8. 明确的停止条件

遇到以下情况，暂停相关 PR 并向用户说明，而不是绕过：

- 需要修改 001–004 或覆盖已发布的 Migration 文件。
- 导入/恢复失败会破坏或静默丢失用户旧数据。
- 需要删除/移动/篡改游戏、Switch SD 卡数据、游戏存档或用户其他原始文件。
- 新接口绕过 LoomaCore/审计/权限边界，或默认把敏感数据发到远端。
- 真实 NX Activity Log JSON 与解析假设不符，且无法通过向后兼容的解析策略解决。
- 大规模数据/巨型图片导致无法建立可接受的资源上限。

日常实现细节、函数命名、组件样式和一般性能优化由 Antigravity 自主决定；不要为微小分歧无限延长评审。

---

## 9. 不可忽略的验收用例矩阵

| 编号 | 输入/动作 | 必须结果 |
|---|---|---|
| A01 | 单用户 Switch 导出，12 款游戏 | 预览、匹配、导入 12 条统计，单位秒准确 |
| A02 | 两 Profile 同一 Title ID 分别 40h、10h | 两个来源，分别展示；不能覆盖成一个 50h |
| A03 | 无匹配作品的 Title ID | 保存待匹配活动，不强制新建 Work |
| A04 | 完全相同 JSON 第二次提交 | 幂等跳过，批次数量与时长不增加 |
| A05 | 新导出从 40h 到 45h | 新快照保留 45h，旧 40h 可追溯，不相加为 85h |
| A06 | 新导出从 85h 变为 2h | 标注重置/异常，保留两条快照，不产生 87h |
| A07 | 先导入新文件，再导入旧文件 | 不把旧导出错误认定为最新统计 |
| A08 | 用户选择“暂不匹配” | 统计仍可恢复、在待认领列表显示 |
| A09 | 关联 Work 后更改 Work 名称 | 设备报告的标题留作来源，主作品名不被覆盖 |
| A10 | 同名不同 Title ID | 用户决定是否映射同 Work，不得静默合并 |
| A11 | 首次通关与重试同操作请求 | 同一操作只产生一条 Experience |
| A12 | 同一款游戏真正二周目 | 允许第二条 Experience，不被固定 Title ID 幂等键阻止 |
| A13 | 无 events/时间 0/未知字段/非法 ID | 兼容有效数据，错误明确且不产生脏写 |
| A14 | 中途发生外键/SQL 写失败 | 来源、映射、批次、快照、审计全部回滚 |
| A15 | 导出并恢复含活动数据的 Vault | 来源身份、映射状态、批次及历史快照一致 |
| A16 | 删除、归档、解绑某个 Work | 不应隐式抹除个人足迹或来源历史 |
| A17 | Steam 与本机观察来源统计重叠 | 默认分来源展示，不伪造精确合计 |
| A18 | AI 非 JSON、超时、拒绝请求 | 没有任何资产/Work 写入；保留错误与待处理状态 |
| S01 | 200 个资产，其中查第 151 个 | 列表翻页和全局搜索都能找到 |
| S02 | 作品详情陈旧快照提交 | CAS 冲突，可重载；无静默覆盖 |
| S03 | 巨型图/损坏图并发缩略图请求 | 有资源上限、可降级、原图不变 |

---

## 10. 给反重力的直接指令

> 本文件是 Looma v0.6 的最终架构实施契约。三个方案 A 全部采纳，但原先三表 DDL 不得直接实施：必须支持多 Profile、未匹配游戏、导入批次、历史快照，并避免误杀二周目足迹。  
> 从最新 Git HEAD 开始，先进行 v0.5.1 独立 hotfix，然后按 A0 → A1 → A2 → A3 实施 Switch MVP。完成且有真实数据验收后，再按独立 PR 考虑 Steam/Windows/AI。  
> 迁移编号与已发布历史保持一致；每个阶段自己运行测试、做临时 Vault UI 验收、提交并给出简洁报告，可以连续推进。只在明确的停止条件触发时暂停请求决策。  
> **最终目标：不动 Switch 游戏文件和存档，也不需要在 PC 上复制游戏，就能把 Switch 游玩记录与现有 Looma Work、Experience 统一起来。**

## 参考依据

- Antigravity 本地源码核验：《Looma v0.6 架构交叉讨论回复》，HEAD `75eb31a`。
- NX Activity Log 导出格式（第三方文档镜像）：https://github-wiki-see.page/m/tallbl0nde/NX-Activity-Log/wiki/Exported-JSON-Format
- NX Activity Log 项目：https://github.com/tallbl0nde/NX-Activity-Log
- Steam Web API 概览：https://partner.steamgames.com/doc/webapi_overview
