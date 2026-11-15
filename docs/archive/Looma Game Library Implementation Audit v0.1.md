# Looma Game Library Implementation Audit v0.1

## 0. 审计目的

本任务不是继续开发功能。

本任务是对当前 Looma 仓库进行一次：

> **Game Library Implementation Audit**

目标是验证上一阶段新增的「个人游戏库」是否已经真正贯通：

```text
真实 Domain
    ↓
真实 Database
    ↓
真实 LoomaCore
    ↓
真实 Scanner
    ↓
真实 GUI
    ↓
真实 CLI
    ↓
真实 MCP
```

尤其要排除以下情况：

```text
UI 有游戏库
    ↓
实际上读取静态 Mock Data
```

或者：

```text
README 宣称支持
    ↓
Domain 没有真实持久化
```

或者：

```text
Game Library 有界面
    ↓
实际上绕过 LoomaCore 直接操作 DB
```

---

# 1. 审计原则

本次审计：

### 不允许：

- 为了通过审计临时修改架构
- 添加与审计无关的新功能
- 用 Mock 数据证明功能存在
- 用 README 描述代替代码证据
- 用截图代替持久化测试
- 用“理论上可以”作为 PASS

### 必须：

- 检查实际 Rust Domain
- 检查 Database Schema / Migration
- 检查 Core Service / Facade
- 检查 Scanner
- 检查 GUI 数据来源
- 检查 CLI
- 检查 MCP
- 检查自动化测试
- 实际运行测试
- 必要时手工操作真实 Desktop App

---

# 2. 最终输出格式

请最终生成一份：

```text
GAME_LIBRARY_AUDIT_REPORT.md
```

每项必须使用：

```text
PASS
PARTIAL
FAIL
N/A
```

并提供：

```text
代码位置
测试方式
实际结果
问题
建议
```

不要只写：

> 已实现。

必须说明：

> 在哪里实现、如何证明、测试结果是什么。

---

# 3. P0：Game Work 是否是真实 Domain

首先检查当前：

```text
Work
```

模型。

确认：

```text
type = game
```

是否是真实的 Domain 类型。

检查：

```text
crates/core
```

以及相关：

```text
Work model
WorkType / EntityType
Progress
Relation
Asset
```

必须确认：

```text
Game ≠ 独立数据库模型
```

而是：

```text
Work
type = game
```

或者当前架构中等价的统一模型。

### 验收

创建：

```text
Test Game
```

保存后：

```text
关闭应用
重新打开
```

仍然存在。

### 结果

```text
[ ] PASS
[ ] PARTIAL
[ ] FAIL
```

---

# 4. P0：Game Library 是否只是 Work(type=game) 的真实 View

检查 Game Library 页面数据来源。

目标：

```text
Game Library
    ↓
LoomaCore
    ↓
Work(type = game)
```

禁止：

```text
Game Library
    ↓
gameMockData
```

或：

```text
Game Library
    ↓
hard-coded JSON
```

或：

```text
Game Library
    ↓
独立 Game Store
```

### 必须提供

实际调用链：

```text
UI Component
    ↓
Frontend API / Tauri Command
    ↓
LoomaCore
    ↓
Database
```

并标明实际代码文件。

---

# 5. P0：Game ↔ Asset 是否真实持久化

这是本次最重要的检查之一。

创建：

```text
Game A
```

然后关联：

```text
D:\TestGames\GameA\
```

或者当前环境下的临时测试目录。

确认数据库中存在真实关系：

```text
Game Work
    ↓
Asset
```

而不是 UI 临时计算：

```text
Game title
    ↓
search folder name
```

### 验收

1. 创建 Game
2. 添加 Local Asset
3. 退出程序
4. 重新打开
5. Game Detail 仍然显示该目录

### 必须提供

```text
Work ID
Asset ID
Relation / Foreign Key / Reference
```

等实际持久化证据。

---

# 6. P0：Directory Asset 是否真实存在

检查当前 Asset 模型。

确认是否能够表达：

```text
File
Directory
```

或者通过现有 URI / metadata 机制表达目录。

优先复用现有 Asset 模型。

禁止为了 Game 单独创造：

```text
GameDirectory
GameInstallPath
GameLocationTable
```

除非有非常充分的架构理由。

### 必须回答

当前 Looma 是如何表达：

```text
E:\Games\GameA\
```

的？

给出：

```text
Domain model
Database schema
实际存储值
```

---

# 7. P0：一个 Game 是否支持多个本地位置

测试：

```text
Game A
├── D:\Games\GameA
├── E:\Archive\GameA
└── F:\Backup\GameA
```

要求：

```text
一个 Work
    ↓
多个 Asset
```

而不是：

```text
game_path = "D:\Games\GameA"
```

只能保存一个路径。

### 验收

添加三个不同位置。

重新启动应用。

三个位置都必须仍然存在。

---

# 8. P0：Scanner 是否真的支持 Game Candidate

检查 Scanner。

必须区分：

```text
Scanner
    ↓
发现目录
    ↓
Candidate
```

和：

```text
Scanner
    ↓
直接创建 Work
```

推荐结构：

```text
Local Directory
      ↓
Scanner
      ↓
Candidate
      ↓
User Confirmation
      ↓
Create / Link Work
```

### 必须回答

Scanner 当前是否真的会发现潜在 Game？

如果会：

```text
检测规则是什么？
```

例如：

```text
executable
directory structure
metadata
name similarity
```

如果不会：

明确：

```text
NOT IMPLEMENTED
```

不能把普通 Asset Scanner 描述成 Game Candidate Detection。

---

# 9. P0：Candidate 是否会污染数据库

测试一个未知游戏目录：

```text
D:\TestGames\UnknownGame\
```

运行扫描。

预期：

```text
Candidate
```

而不是直接：

```text
Work(type=game)
```

要求用户确认后才能建立 Work。

### 验收

扫描前：

```text
Game count = N
```

扫描后：

```text
Game count = N
Candidate count = N+1
```

用户确认后：

```text
Game count = N+1
```

如果当前设计不是 Candidate，而是其他等价机制，请解释。

---

# 10. P0：Scanner 是否使用危险的来源判断

确认 Game Candidate Detection 没有依赖：

```text
破解文件
破解组
破解补丁
特定 DLL
下载来源
盗版网站
```

Looma 只应该判断：

> 这个本地目录是否看起来像一个游戏资产。

不能判断：

> 这个游戏是否合法。

---

# 11. P0：Game Progress 是否复用现有 Progress

确认：

```text
Game
    ↓
Progress
```

而不是：

```text
GameProgressTable
```

至少验证：

```text
planned
in_progress
completed
paused
dropped
```

等当前状态是否真实持久化。

如果当前已有：

```text
position
position_type
```

验证 Game 是否能够使用。

---

# 12. P0：Game Collection 是否复用 Collection

验证：

```text
Game
    ↓
Collection
```

例如：

```text
我的单机游戏
正在玩
想玩
已通关
```

确认 Collection 是通用模型。

禁止：

```text
GameCollectionTable
```

成为新的平行 Domain。

---

# 13. P0：ExternalReference 是否真实复用

验证：

```text
Game
    ↓
ExternalReference
```

例如：

```text
Official Website
Steam
Wiki
```

确认 URL 是真实持久化数据。

不要创建：

```text
GameUrl
GameWebsite
GameStoreLink
```

等专用模型。

---

# 14. P0：GUI 是否通过 LoomaCore

这是架构审计的核心。

验证：

```text
GUI
 ↓
LoomaCore
 ↓
Database
```

而不是：

```text
GUI
 ↓
SQLite
```

或者：

```text
GUI
 ↓
Game-specific DB API
```

请提供实际代码调用链。

---

# 15. P0：CLI 是否能够访问 Game

检查 CLI。

至少验证：

```text
list games
get game
```

或者通过通用 Work API：

```text
work list --type game
work get <id>
```

均可。

重点不是命令名字。

重点是：

```text
CLI
 ↓
LoomaCore
 ↓
同一份数据
```

---

# 16. P0：MCP 是否能够访问 Game

验证 MCP。

AI 至少应该能够通过现有通用能力查询：

```text
Game Works
Progress
Assets
ExternalReferences
Collections
```

不要求创建大量：

```text
game-specific MCP tools
```

优先复用：

```text
search
get_work
get_asset
get_progress
```

等通用能力。

---

# 17. P0：三端一致性

这是本次最终验收。

建立：

```text
Game A
```

关联：

```text
Local Asset
Progress
Reference
Collection
Memory
```

然后分别：

```text
GUI 查询
CLI 查询
MCP 查询
```

要求得到同一份 Domain 数据。

目标：

```text
                LoomaCore
              /     |     \
            GUI    CLI    MCP
              \     |     /
               Same Domain
                    |
                  SQLite
```

---

# 18. P0：重启持久化

必须做：

```text
Create
 ↓
Save
 ↓
Close App
 ↓
Restart
 ↓
Read
```

验证：

```text
Game
Asset
Progress
Reference
Collection
```

全部仍存在。

这是排除：

```text
React state
mock state
memory cache
```

最直接的方法。

---

# 19. P1：失效本地 Asset

测试：

```text
Game A
└── D:\TestGames\GameA
```

然后将测试目录移动或删除。

重新扫描 / 打开 Game。

系统应该能够表达：

```text
Missing
```

而不是：

```text
Asset disappeared
```

建议：

```text
Asset
status = missing
```

或者当前架构的等价表达。

不要自动删除 Work。

---

# 20. P1：多个游戏目录的 Fragment Detection

测试：

```text
D:\Games\GameA
E:\Archive\GameA
F:\Backup\GameA
```

检查系统是否：

```text
提示可能属于同一 Work
```

如果尚未实现：

```text
PARTIAL
```

即可。

不要为了通过审计临时加入复杂算法。

---

# 21. P1：Game Cover

如果当前实现了封面：

验证：

```text
Game
 ↓
Asset
 ↓
Cover
```

是否是真实 Asset Reference。

如果只是 UI URL：

```text
PARTIAL
```

即可。

---

# 22. P1：打开游戏目录

如果 UI 有：

```text
打开目录
```

验证它使用的是：

```text
真实 Asset Path
```

而不是：

```text
hard-coded path
```

同时确认不会把路径复制到第二套 Game 数据库。

---

# 23. P1：游戏库统计

如果当前存在：

```text
42 Games
12 未开始
4 游玩中
18 已完成
```

验证这些数字是否来自真实 Domain 查询。

禁止：

```text
const stats = {...}
```

或：

```text
mockStats
```

---

# 24. P1：静态数据污染检查

全局搜索：

```text
mock
fixture
demo
sample
fake
hardcoded
static
```

重点检查：

```text
Game Library
Game Detail
Progress
Game Statistics
Game Candidate
```

如果存在 Demo Data：

必须说明：

```text
Production Data Path
Demo Data Path
```

二者不能混淆。

---

# 25. P1：数据库 Migration

检查是否新增：

```text
migration
schema
index
constraint
```

如果 Game Library 完全复用了已有 Schema：

这是最理想情况。

报告：

```text
No new Game-specific schema required
```

如果新增 Schema：

必须解释：

```text
为什么现有 Work / Asset / Relation 无法表达？
```

---

# 26. P1：Domain 是否被 Game 污染

检查是否出现：

```text
GameService
GameRepository
GameTable
GameEntity
GameAssetTable
GameProgressTable
```

如果这些只是 Application/UI 层便利封装：

可以接受。

如果它们形成了：

```text
第二套 Game Domain
```

则标记：

```text
ARCHITECTURE CONCERN
```

---

# 27. P1：Local-First 边界

确认 Game Library 不会默认联网。

正常流程：

```text
本地目录
 ↓
Scanner
 ↓
Looma
```

External Reference：

```text
保存 URI
 ↓
用户主动点击
 ↓
系统浏览器
```

不要出现：

```text
后台自动抓取网站
自动上传本地目录
自动上传游戏文件
```

---

# 28. P1：大目录性能

准备一个模拟的大型游戏目录。

例如：

```text
10,000 files
```

验证：

```text
Game Library
```

不会尝试一次性加载所有文件。

理想表现：

```text
Game
├── Local Path
├── Asset Count
└── Open Asset Browser
```

而不是：

```text
Game
└── 10,000 个文件全部渲染
```

---

# 29. P2：暂不要求实现

以下功能本次不作为 FAIL：

```text
Steam API
Epic API
自动启动游戏
Playtime Tracking
Achievement Tracking
Save Game Management
Emulator Integration
Online Metadata Import
OCR
Semantic Search
AI Game Agent
Cloud Game Sync
```

如果已经实现，可以记录。

但不要因为没有这些功能而判 FAIL。

---

# 30. 最重要的真实验收 Case

请完整执行下面的 Scenario。

## Scenario A：本地游戏

建立：

```text
D:\LoomaAudit\Games\GameA\
```

目录中放置少量测试文件。

执行：

```text
Scanner
 ↓
发现 Candidate
 ↓
用户确认
 ↓
创建 Game Work
 ↓
关联 Local Asset
```

验证：

```text
GUI
CLI
MCP
```

都能读取。

---

# 31. Scenario B：多个位置

建立：

```text
D:\LoomaAudit\Games\GameA\
E:\LoomaAudit\Archive\GameA\
```

关联到：

```text
同一个 Game Work
```

重启应用。

验证两个位置仍然存在。

---

# 32. Scenario C：Game + Progress

设置：

```text
Game A
status = in_progress
position = 主线第三章
```

关闭应用。

重新打开。

验证：

```text
GUI
CLI
MCP
```

均能读取。

---

# 33. Scenario D：Game + Reference

添加：

```text
Official Website
```

或者测试 URI：

```text
https://example.com
```

验证：

```text
Game Detail
```

中真实显示。

关闭重启后仍存在。

---

# 34. Scenario E：Game + Collection + Memory

建立：

```text
Game A
├── Collection
│   └── 我的单机游戏
│
└── Memory
    └── 第一次游玩记录
```

验证完整关系。

---

# 35. Scenario F：删除 / 移动本地目录

将：

```text
GameA
```

目录移动。

重新打开 Looma。

验证：

```text
Game Work
```

仍存在。

Local Asset 显示：

```text
Missing
```

或者当前系统等价状态。

---

# 36. 必须给出的代码证据

最终报告至少列出：

```text
Domain Model:
<file>

Database Schema / Migration:
<file>

Core Service:
<file>

Asset Relation:
<file>

Scanner:
<file>

Candidate:
<file>

GUI:
<file>

CLI:
<file>

MCP:
<file>

Tests:
<file>
```

如果某项不存在：

直接写：

```text
NOT IMPLEMENTED
```

不要用 README 代替。

---

# 37. 最终评分

使用：

```text
P0
├── Domain
├── Persistence
├── Asset Relation
├── Multi-location
├── Scanner
├── Core
├── GUI
├── CLI
├── MCP
└── Restart Persistence
```

评分：

```text
PASS      = 真实实现 + 有测试证据
PARTIAL   = 有部分实现，但链路不完整
FAIL      = 宣称支持，但实际不存在 / Mock
N/A       = 当前阶段明确不需要
```

最终给出：

```text
P0 Score: X / 10
P1 Score: X / 7
Overall: PASS / PARTIAL / FAIL
```

---

# 38. 最终结论必须回答 5 个问题

### Q1

> Game Library 是真实 Domain 功能，还是 UI Mock？

### Q2

> 一个 Game 是否真正能够关联一个或多个本地目录？

### Q3

> Scanner 是否真正能够发现潜在 Game Candidate？

### Q4

> GUI / CLI / MCP 是否共享同一个 LoomaCore 数据链路？

### Q5

> 重启 Looma 后 Game / Asset / Progress / Reference / Collection 是否仍然存在？

---

# 39. 最终架构验收目标

我们最终希望看到：

```text
                     Looma
                       │
                    LoomaCore
                       │
        ┌──────────────┼──────────────┐
        ↓              ↓              ↓
       GUI            CLI            MCP
        │              │              │
        └──────────────┼──────────────┘
                       ↓
                    Domain
                       │
              ┌────────┼────────┐
              ↓        ↓        ↓
             Work     Asset   Memory
              │        │
              │        ↓
              │   Local Directory
              │
              ├── Progress
              ├── Collection
              ├── ExternalReference
              └── Timeline
```

其中：

```text
Game
```

只是：

```text
Work(type = game)
```

而：

```text
Game Library
```

只是：

```text
Work(type = game)
```

的专门用户视图。

---

# 40. 本次审计的最终目的

不是证明：

> “我们做了一个很厉害的 Game Manager。”

而是证明：

> **Looma 原有的 Personal Digital OS 架构，能够自然容纳“本地散落游戏库”这个真实需求，而不需要新增一套平行系统。**

如果最终能够做到：

```text
本地文件散落
       ↓
Scanner
       ↓
Candidate
       ↓
用户确认
       ↓
Work(type=game)
       ↓
Asset
       ↓
Progress / Memory / Collection / Reference
       ↓
GUI / CLI / MCP
```

则说明这次 Game Library Extension **真正符合 Looma 原始架构目标**。

如果其中任何环节依赖：

```text
Mock
Hardcoded Data
独立 Game DB
UI-only State
绕过 Core
```

必须明确标记出来。

**本次任务只审计，不新增功能。**