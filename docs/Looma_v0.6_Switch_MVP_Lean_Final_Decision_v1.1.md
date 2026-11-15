# Looma v0.6｜Switch 游玩记录接入：轻量版最终实施决议 v1.1

> **文档定位**：替代 `Looma_v0.6_Final_Architecture_Decision_v1.0.md` 中过重的 Switch MVP 持久化设计；供 Antigravity 直接实施。  
> **日期**：2026-11-15  
> **目标用户**：单人、本地 Windows、单个 Looma Vault、个人 Switch 1（可能有多个地区/Profile 账号）。  
> **原则**：够用、容易使用、可解释、能恢复、不碰原件；不为了未来不确定的规模提前设计设备管理平台。  
> **执行权限**：不再开展整轮架构讨论；按本文件的验收标准分阶段实施，有真正的数据安全阻断再报告。

## 0. 本轮明确裁定

采纳 Antigravity 的**一张活动主表方案**，并修正两个不可忽略的持久化细节：**来源 Profile 身份**及**唯一约束的业务键**。不建 `activity_sources`、`external_game_identities`、`activity_import_batches`、`playtime_observations` 四表体系，也不设长期未匹配游戏池。

| 问题 | 最终决议 |
|---|---|
| Work/Experience | 沿用已有 Work/Entity、ExperienceEntry，不把时长写进 WorkMetadata |
| 来源 | 一张 `game_activity_records` 表；一个稳定的 `source_key` 字符串即可，无设备注册 UI |
| Profile | 单 Profile 自动选中；多个 Profile 在导入预览选择，按 Profile 保留事实，不强行相加 |
| 未识别游戏 | 在本次预览里选择“关联 Work / 新建 Work / 跳过”，不保留持久孤儿池 |
| 时间数据 | 同一来源同一游戏只保存**最新确认累计值**，附历史峰值以识别重置；不保存每次导入的历史快照 |
| 幂等 | 相同业务键更新而非累加；无变化不重复写入/不增加时长；同源旧导出不默默覆盖新值 |
| 通关 | 活动导入不自动推断通关；用户点击时才生成 Experience，默认日期未知 |
| 多平台 | 各来源时长分开显示，不自动求和（Steam 与 Windows 监测可能重叠） |
| AI | Switch MVP 之后独立提供可选文本 JSON 建议，不使用 Tool Calling 或模型直接写库 |
| v0.5.1 | CAS、分页、图片解码限流、错误可见性独立修复，不污染 Switch Migration |

**必须避免的两种极端**：既不建立单人无需的四表活动平台，也不以“只有一个用户”为由删除能防止真实数据覆盖的基本约束。

## 1. 建议的最小数据库契约

下列 SQL 是建议的**实现草案**，请核实仓库实体主键、时间序列化与迁移规则后定稿；历史迁移 001–004 不可修改。若 v0.5.1 已占用 005，应自动顺延本迁移编号，不允许重名。

```sql
CREATE TABLE IF NOT EXISTS game_activity_records (
    id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES entities(id) ON DELETE RESTRICT,

    -- 来源提供方与实际账号/Profile 的稳定键；不是多租户注册系统
    provider TEXT NOT NULL,           -- switch_nx | steam | pc_observer（后两者仅预留取值）
    source_key TEXT NOT NULL,         -- Switch users[].id 的稳定派生值；不使用昵称
    external_id TEXT NOT NULL,        -- Switch Title ID（规范为 16 位 hex）
    source_label TEXT,                -- 用于显示：Switch（美服号）
    title_hint TEXT,                  -- 导出源中显示的游戏名称，仅作提示

    playtime_seconds INTEGER NOT NULL CHECK (playtime_seconds >= 0),
    peak_seen_seconds INTEGER NOT NULL CHECK (peak_seen_seconds >= 0),
    launch_count INTEGER CHECK (launch_count IS NULL OR launch_count >= 0),
    first_played_at TEXT,
    last_played_at TEXT,
    source_exported_at INTEGER,       -- 来源导出时间，Unix 秒；与导入时间严格区分
    last_import_sha256 TEXT,          -- 最近一次已接受源文件哈希，非全量历史表
    updated_at TEXT NOT NULL,

    CHECK (length(provider) > 0 AND length(source_key) > 0 AND length(external_id) > 0),
    UNIQUE(provider, source_key, external_id)
);
CREATE INDEX IF NOT EXISTS idx_game_activity_work
    ON game_activity_records(work_id);
```

### 为什么不使用 Antigravity 原始的 `UNIQUE(work_id, platform, external_id)`？

- 如果将同一个 Switch Title ID 错误地关联到两份不同 Work，旧约束会允许两行重复的事实记录。**业务唯一性应由外部来源决定，不由可编辑的 Work 映射决定**。
- 如果多个 Switch Profile 玩同一个 Title ID，不能被看成同一条记录，否则统计互相覆盖；`source_key` 解决这个问题，完全不需要 `device_id` 注册表。
- 两个不同 Title ID 可以通过各自的记录关联同一个 Work（例如地区版本），同时保持源身份可追踪。
- `ON DELETE RESTRICT` 防止误删 Work 时静默级联抹除游玩记录。若用户明确选择“永久删除作品及关联活动”，由 Core 提供单独确认流程，而不是数据库暗中清除。

### 数据量与复杂度

每个来源 Profile / 游戏最多一条当前记录。个人 Vault 通常数十至数千行，一次 `SELECT ... WHERE work_id = ?` 即可显示。不存在为浏览详情而进行四表 JOIN 的负担。

`peak_seen_seconds` 只代表“历史曾导入过的最大报告值”，**不是**用户实际总游玩时长。若最新 2h、历史峰值 85h，UI 显示“当前源报告 2h；曾报告 85h，可能重置”，绝不能自动补加为 87h。

## 2. Switch JSON 导入流程

```text
选择 NX Activity Log JSON
  → 本地解析、检测导出版本/大小/标题 ID、选择 Profile
  → 匹配现有 Work（已有稳定映射优先；标题匹配只给建议）
  → 在同一预览页确认：关联已有 / 新建 / 跳过
  → 对时间倒退、时长减少、重复导入给出一次性提示
  → 单个 SQLite 事务：创建必要 Work + Upsert 活动记录 + 审计
  → 游戏详情出现 Switch 分来源时长，可选手动记录通关
```

用户体验规定：

1. JSON 只有一个 Profile：自动选择，不增加操作。存在两个或多个 Profile：显示简单多选，记住**上次本地选择**（若不实现记忆，至少导入预览一眼能选）；不需要注册设备、录入序列号或配置租户。
2. 从 `users[].id` 派生稳定 `source_key`，不能用可变的 Profile 昵称。用户属于同一个人不意味着不同 Profile 的累计时长可以直接覆盖或合计。
3. 同名候选仅作**预览建议**；同 Title ID 已有明确记录时直接复用既有 Work 关联。新游戏默认建议新建 Work，允许批量确认；试玩版/Demo 可以直接跳过。
4. 解析 `summary.playtime` 时**单位为秒**；`firstPlayed`/`lastPlayed` 的 0 值应为未知；允许无 `events` 的摘要导出。未知导出版本必须明确说明兼容情况。
5. 预览到提交间应验证文件未被替换（对选定文件内容做哈希或持有已校验的解析结果）；所有写入必须在一个 Core → Repo 事务中完成，失败后原 Vault 不变。
6. 本地读取 JSON 并不等于移动或修改 Switch SD 卡内容；绝不从 Looma 修改或删除游戏、存档和导出原件。

### 同一游戏再次导入的判定

- **同一快照 / 相同数据**：不累加、不重复建 Work，尽可能不产生无变化的数据库写入。
- **较新的统计值变大**：用较新的**绝对累计秒数**替换当前值，`peak_seen_seconds = max(old_peak, new_seconds)`；不得使用 `old + new`。
- **较新的统计值变小**：显示可能重置/异常警告；用户确认后更新最新值并保留历史峰值；取消则保留旧值。
- **旧导出晚于新导出被导入**：根据 `source_exported_at` 提示旧数据，不得静默回退。导出时间无效、时钟错乱时允许用户明确覆盖，但不得伪装为“最新”。
- **同一文件重新选择其他 Profile**：只处理该 Profile 对应的业务键，不应因相同文件 SHA 将整个文件全局禁止再次导入。
- **改绑 Work**：用户明确选择更正映射后，更新对应记录的 `work_id`，不新建第二条同一业务键的游玩事实。

### 对旧数据和误操作的保护

不需要长期的导入批次与 Observation 表，但必须沿用 Looma 现有的审计记录。对重大覆盖（例如 85h → 2h）记录旧值、新值、来源与用户确认；不要误以为现有审计一定已经包含所有字段，需实际检查记录能力并补足必要内容。

## 3. 作品状态、足迹、成就仍然分离

- `WorkMetadata`：游戏作品、主观评分、当前状态等。
- `game_activity_records`：NX Activity Log / 后续 Steam / PC 观察器报告的游玩时长、次数和最近游玩；这些是**来源记录**，不是通关证据。
- `ExperienceEntry`：用户手动确认的通关、二周目、全收集等个人经历。点击“记为通关”才产生；时间默认未知，可填写年月/年份。
- 成就系统不在 Switch MVP 中强行实现；以后 Steam 官方成就、自定义目标分别讨论，不伪造 Switch 官方成就。

**Experience 幂等语义**：同一次按钮提交的重试复用 request_id；下一次真实通关生成新的 request_id。不可永久使用 `(Profile, TitleID)` 作为 Experience 幂等键。

## 4. AI 识别：保持独立的可选功能

Switch MVP 稳定后，单独开发游戏目录/EXE 的**文本识别建议**：

1. 本地规则先判断是否需要 AI；用户主动批准最小必要元数据发送。
2. 模型通过普通单轮文本请求返回 JSON，不依赖 Tool Calling / Tool Response，也不假设网页 Gemini 反代实现了 JSON Schema 输出协议。
3. 严格校验 JSON、字段范围与证据；失败仅提示，不改库。
4. 建议先在 UI 临时待审核；需要跨会话保存时才考虑专用持久化，不强行扩建通用 Agent 运行时。
5. 用户确认后由 LoomaCore 去重/建档或绑定，模型不直接写数据库。

## 5. v0.5.1 尾项单独处理，不扩大 Switch 模型

四项仍独立：旧 Entity 更新覆盖、缩略图解码上限/并发、局部 IPC 吞错、资产页翻页。正式交付 v0.6 前应修复并通过测试。

CAS 方案：**优先验证 `UPDATE entities ... WHERE id = ? AND updated_at = ?` + affected_rows == 1** 是否可以准确拒绝过期提交。Looma 的 Desktop/CLI/MCP 是独立进程，`Mutex<Connection>` 只管单进程连接，不能防止过期 UI 快照。若现有 `updated_at` 精度和更新时间行为无法可靠区分连续更新，应修正时间更新方式，或采用极小的整数版本字段；**不要为了讨论版本字段就预先发起大型迁移**。

## 6. 执行顺序与验收条件

### S0 — v0.5.1 稳定性（独立 PR）

- 修好四项既有尾项，禁止修改真实用户资产。
- 验收：旧快照并发更新冲突、200 文件分页、坏图片/巨图有界解码、IPC 失败显式报错。

### A0 — 单表 Migration + 备份恢复

- 最小模型、索引、Core/Repo API、备份导出与恢复字段、兼容旧 Vault。
- 验收：旧 Vault 升级；FK 生效；同 TitleID 多 Profile 两条记录；一个来源身份只能对应一个 Work；备份恢复后数据一致。

### A1 — Switch JSON 解析与预览

- 只读解析、脱敏真实夹具、多 Profile 选择、标题匹配、同一导出重复检测、时长变小/导出倒退警告。
- 验收：单/多 Profile、空 events、中文/日文、无效时间、错误 TitleID、错误文件、不同时区。

### A2 — 原子导入与游戏详情

- 已确认的映射/新 Work 与活动记录一次事务提交；展示 Switch 时长来源卡片，支持重新映射。
- 验收：导入两次不累加；85h→90h 更新为 90h；90h→2h 要求确认且保留峰值；使用旧导出不静默覆盖；事务中途失败全回滚；不动原始文件。

### A3 — 通关足迹闭环

- 直接调用现有 Experience，在同一 Work 记录通关、二周目、日期未知；不自动根据时间猜测。
- 验收：无截图可通关；再次真实通关不被永久幂等键拦截；时光轴只展示确实记录的个人足迹。

### 后续可选（独立 PR）

- Steam 只读、Windows 进程估算时长；先验证来源准确性与重叠，再决定实现。
- AI 目录识别单轮文本 JSON + 人工审核；**不**内置完整 LLM Agent，不新增 Switch-MVP 必须依赖的网络服务。

### 停止条件

- 必须修改已发布的 001–004 Migration；出现真实 Vault 数据丢失风险；扫描/导入需要修改原游戏文件或存档；备份恢复不能覆盖新表；绕过 Core 直接写库；未经允许把本地元数据发到反代。

## 7. 给 Antigravity 的最终执行指令

> 以本文件替代 v1.0 文件中较重的四表 Switch MVP 契约。采纳单表当前快照 + 轻量来源身份 + 严格唯一键；不要再做完整设备/Profile 注册流程、孤儿待认领库、导入批次表或不可变时序快照表。  
> 先处理独立 v0.5.1，再按 A0→A3 实施。各阶段独立提交、独立测试，完成可继续，不用重新开展宏观架构讨论；只在触发停止条件时汇报。  
> 如果仓库已有等价组件，优先复用，不照搬本文造成重复实现；任何数据层设计变更必须同步核实导出、恢复、迁移与权限门面。

---

**最重要的用户体验验收**：我从 Switch 导出 JSON，拖给 Looma，预览里确认哪几个游戏该关联或新建，点击一次同步。在游戏库能看到来源明确的 Switch 游玩时长，并能随手补录通关。下一次导入不会重复增加时长；即使 Switch 不联网、Looma 不连接 AI，也能完成全过程。
