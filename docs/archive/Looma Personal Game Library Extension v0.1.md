# Looma Personal Game Library Extension v0.1

> **性质：小型 Domain / UI Extension**
>
> 本补丁建立在当前 Looma 已有的：
>
> - Work
> - Progress
> - Asset
> - ExternalReference
> - Memory
> - Collection
> - Timeline
> - Relation
> - LoomaCore
> - Reference Mode
>
> 之上。
>
> **不要重新设计 Game Domain。**
>
> 不创建独立的 Game Database。
>
> 不创建游戏平台。
>
> 不实现游戏下载、来源聚合、破解来源、启动器下载或任何第三方获取机制。
>
> 本补丁只解决一个非常实际的问题：
>
> > 用户本地存在大量分散在不同硬盘、不同目录中的单机游戏，希望 Looma 将它们整理成一个统一的个人游戏库。

---

# 1. 为什么需要这个 Extension

当前 Looma 已经支持：

```text
Work
type = game
```

也支持：

```text
Progress
Assets
ExternalReference
Collection
Memory
Timeline
```

因此从数据模型上已经能够表示游戏。

但是现实使用中还有一个明显问题：

```text
D:\Games\
D:\Game\
E:\SteamLibrary\
E:\OldGames\
F:\Downloads\Games\
F:\Archive\
```

用户可能有大量游戏分散在不同位置。

例如：

```text
D:\Games\GameA\
E:\OldGames\GameB\
F:\Archive\GameC\
```

Looma 应该能够把这些文件：

```text
本地目录
   ↓
Asset Scanner
   ↓
识别为潜在 Game Work
   ↓
加入个人游戏库
```

而不是要求用户自己手动维护一个巨大列表。

---

# 2. 核心定位

Looma 的 Game Library 应该理解为：

> **“我的游戏资产索引”**

而不是：

> “游戏平台”

因此：

```text
Game Library
    ↓
记录我有什么
记录它在哪里
记录我玩到哪里
记录我是否完成
记录我的截图/视频
记录相关网页
记录我的个人笔记
```

不负责：

```text
游戏下载
游戏分发
破解来源
下载链接聚合
第三方安装器
在线游戏平台
```

---

# 3. 数据模型

继续使用：

```text
Work
type = game
```

一个游戏：

```text
Game Work
│
├── Progress
├── Assets
├── ExternalReferences
├── Memories
├── Collections
├── Timeline
└── Relations
```

例如：

```text
Work
└── NieR:Automata
    type = game

    Progress
    ├── status = in_progress
    └── position_type = custom

    Assets
    ├── game directory
    ├── screenshots
    └── recordings

    ExternalReferences
    ├── official website
    └── wiki

    Memories
    └── 游戏体验记录

    Collection
    └── 我的单机游戏

    Timeline
    └── 2026-10-03 开始游玩
```

---

# 4. 不创建 GameTable

禁止：

```text
GameTable
GameLibraryTable
GameInstallationTable
GameLauncherTable
```

作为新的核心领域。

继续使用：

```text
Work
Asset
Relation
Progress
Collection
```

---

# 5. 新增概念：Game Library View

建议增加一个 UI View：

```text
个人游戏库
```

它不是新的数据表。

只是：

```text
Work
WHERE type = game
```

的专门视图。

---

# 6. 游戏库 UI

推荐：

```text
个人游戏库

┌─────────────────────────────────────────┐
│ 我的游戏                         [扫描] │
├─────────────────────────────────────────┤
│                                         │
│ 🎮 NieR:Automata                        │
│ 本地资源：已关联                        │
│ 状态：游玩中                            │
│                                         │
│ 🎮 Persona 5                            │
│ 本地资源：已关联                        │
│ 状态：未开始                            │
│                                         │
│ 🎮 某游戏                                │
│ 本地资源：已关联                        │
│ 状态：已完成                            │
│                                         │
└─────────────────────────────────────────┘
```

具体 UI 风格遵循当前 Looma。

不要为了这个功能重新设计整个导航。

---

# 7. 游戏库筛选

建议支持：

```text
全部
未开始
游玩中
已完成
暂停
放弃
本地有资源
无本地资源
```

以及：

```text
硬盘
目录
Collection
```

例如：

```text
全部游戏
├── D:
├── E:
└── F:
```

---

# 8. 最重要功能：Local Game Location

这里是本补丁真正需要补的地方。

一个 Game Work 可以关联一个或多个本地目录/资产。

例如：

```text
NieR:Automata
│
└── Local Assets
    └── E:\Games\NieR_Automata\
```

另一个：

```text
Game B
│
├── D:\Games\GameB\
└── F:\Backup\GameB\
```

允许多个 Asset。

---

# 9. 不重复保存路径

不要：

```text
Work
├── game_path
└── Asset
    └── path
```

而应该：

```text
Work
    ↓
Asset
    ↓
path
```

Work 只通过关系知道：

> “这个游戏有哪些本地资产。”

---

# 10. Directory Asset

如果当前 Asset 模型只支持文件：

```text
file
```

则本补丁可以增加：

```text
AssetKind
├── File
└── Directory
```

或者使用现有 Asset URI / metadata 机制表达目录。

优先复用现有模型。

**不要重新设计 Scanner。**

---

# 11. 游戏目录识别

可以增加非常轻量的：

```text
Game Candidate Detection
```

用于扫描后提示：

```text
检测到可能的游戏目录：

E:\Games\ExampleGame
```

但必须是：

> 建议

而不是：

> 自动创建

---

# 12. 不要依赖破解特征

Game Candidate Detection 不允许依赖：

```text
破解文件
补丁文件
特定破解组
特殊 DLL
破解启动器
```

也不要尝试判断：

```text
正版
盗版
破解版
```

Looma 只关心：

```text
这是一个本地游戏目录
```

---

# 13. 推荐识别信号

可以使用普通的本地文件结构信号，例如：

```text
目录中存在可执行文件
目录包含大量游戏资源
存在明显的游戏元数据
目录名称与 Work Title 高度相似
```

但识别结果必须是：

```text
Candidate
```

例如：

```text
Possible Game

E:\Games\NieR_Automata

[创建游戏] [忽略]
```

---

# 14. 扫描器不要自动污染数据库

非常重要：

```text
Scanner
 ↓
Candidate
 ↓
User confirmation
 ↓
Work
```

而不是：

```text
Scanner
 ↓
1000 个目录
 ↓
自动创建 1000 个 Game Work
```

否则 Game Library 很快会变成垃圾数据。

---

# 15. Game Work 与 Asset 的关联

确认以后：

```text
Work
└── Game
    │
    └── Asset
        └── E:\Games\ExampleGame\
```

如果目录中已有文件 Asset：

```text
Work
└── Game
    ├── Directory Asset
    ├── exe
    ├── screenshot
    └── other files
```

根据当前 Asset 系统能力决定展示粒度。

---

# 16. 避免扫描游戏目录造成巨大噪音

这是一个非常重要的 UX 问题。

大型游戏目录可能有：

```text
10000+
```

甚至更多文件。

不要默认在：

```text
Game Detail
```

里把全部内部文件展开。

应该：

```text
Game
│
├── 本地位置
│   └── E:\Games\ExampleGame
│
├── 资产数量
│   └── 12342
│
└── [打开目录]
```

用户需要的时候再进入 Asset Browser。

---

# 17. Game Cover

如果当前 Asset 中已经存在：

```text
poster
cover
screenshot
```

允许用户选择一个：

```text
Cover Asset
```

作为游戏卡片封面。

不要强制从互联网下载封面。

---

# 18. 外部 Reference

继续使用：

```text
ExternalReference
```

例如：

```text
Game
└── NieR:Automata

External References
├── Official Website
├── Steam
└── Wiki
```

这里只记录用户自己提供或已有的 URI。

不要增加：

```text
GameStore
GameDownload
CrackSource
```

---

# 19. Progress

继续使用现有通用 Progress。

游戏可以：

```text
status = planned
status = in_progress
status = completed
status = paused
status = dropped
```

游戏进度：

```text
position_type = custom
```

例如：

```text
progress = "主线第三章"
```

或者：

```text
progress = "42%"
```

当前版本不要建立：

```text
Quest
Achievement
SaveGame
Playtime
```

等复杂结构。

---

# 20. Playtime

如果当前已有 Timeline：

可以通过 Timeline 记录：

```text
2026-10-03
开始游玩

2026-10-05
继续游玩
```

但本版本**不要实现自动游戏时间统计**。

以后可以作为独立插件 / Integration。

---

# 21. Collection

Game Library 可以天然使用：

```text
Collection
```

例如：

```text
我的游戏
├── 想玩
├── 正在玩
├── 已通关
├── RPG
├── 2026
└── Favorites
```

不要建立：

```text
GameCategory
GameGenreTable
GameListTable
```

---

# 22. 一个特别有用的 Collection

建议预置：

```text
我的单机游戏
```

但它应该只是普通 Collection。

不是特殊数据库。

用户以后可以删除。

---

# 23. “正版 / 非正版”不要进入 Domain

Looma 不需要保存：

```text
license_type
piracy_status
crack_type
release_group
```

这些信息与 Looma 的核心目标无关。

Looma 只记录：

```text
我有这个 Work
我有这个本地 Asset
我把它放在哪里
我玩到哪里
```

这样 Domain 也更加干净。

---

# 24. Game Library 的真正价值

最终：

```text
                    Game Library
                          │
             ┌────────────┼────────────┐
             ↓            ↓            ↓
           Works        Assets      References
             │            │            │
             ↓            ↓            ↓
          游戏档案      本地位置       外部入口
             │
       ┌─────┼─────┐
       ↓     ↓     ↓
    Progress Memory Collection
```

例如：

```text
我的游戏库

42 Games

├── 12 未开始
├── 4 游玩中
├── 18 已完成
├── 8 暂停
│
├── D: 18
├── E: 15
└── F: 9
```

---

# 25. Game Library Search

支持：

```text
游戏名称
目录名称
Asset 路径
External Reference
Collection
Status
```

例如：

```text
搜索：
nier
```

可以找到：

```text
NieR:Automata
E:\Games\NieR_Automata
Steam Reference
我的 RPG Collection
```

---

# 26. Duplicate / Fragment Detection

这是一个非常值得做的小功能。

例如：

```text
D:\Games\GameA
E:\Archive\GameA
F:\OldGames\GameA
```

Looma 可以提示：

```text
可能属于同一游戏：

GameA

发现 3 个本地位置
```

然后允许：

```text
[关联到现有 Work]
[创建新 Work]
[忽略]
```

**不要自动合并。**

---

# 27. 多位置游戏

一个 Work 可以关联：

```text
多个 Asset
```

因此：

```text
Game A
├── D:\Games\GameA
├── E:\Backup\GameA
└── F:\Archive\GameA
```

都是合法的。

这比：

```text
game_path = "D:\Games\GameA"
```

强很多。

---

# 28. GUI 快速操作

Game Detail 推荐：

```text
[打开游戏目录]
[扫描此目录]
[更新进度]
[添加外部链接]
[添加到收藏]
```

其中：

```text
打开游戏目录
```

只是调用系统文件管理器。

本版本不需要：

```text
[启动游戏]
```

如果以后要做启动器，也应该作为独立能力。

---

# 29. CLI

继续复用：

```text
looma work
looma asset
looma collection
```

如果需要专门入口，可以增加：

```text
looma game list
```

但它应该只是：

```text
looma work list --kind game
```

的便利别名。

不要让 CLI 产生第二套 Game Domain。

---

# 30. MCP

MCP 不需要新增大量 Game 专用工具。

AI 应该能够：

```text
查询我的游戏
查询游戏本地位置
查询游戏进度
查询游戏 Collection
查询游戏 References
```

通过：

```text
search
get_work
get_asset
get_progress
```

等通用能力完成。

---

# 31. 一个非常好的 AI 场景

以后可以问：

> “我电脑上有哪些还没玩的单机游戏？”

Looma 返回：

```text
12 个：

Game A
E:\Games\GameA

Game B
D:\Games\GameB

Game C
F:\Archive\GameC
```

再问：

> “哪些游戏已经有本地资源但我还没建立作品档案？”

则返回：

```text
Possible Candidates
├── Game A
├── Game B
└── Game C
```

然后：

> “把 Game A 和 Game B 建立成我的游戏档案。”

AI 才调用写入能力。

这就是这个扩展真正有价值的地方。

---

# 32. 扫描流程

推荐：

```text
Scanner
   ↓
Local Files / Directories
   ↓
Candidate Detection
   ↓
Possible Game Candidates
   ↓
User Confirmation
   ↓
Create / Link Work
   ↓
Work ↔ Asset
```

不要：

```text
Scanner
 ↓
LLM
 ↓
自动创建
```

当前版本不需要 LLM。

---

# 33. Game Library 首页

可以增加：

```text
我的游戏

┌─────────────────────────────────────┐
│ 42 Games                             │
│                                     │
│ 12 未开始   4 游玩中   18 已完成     │
│                                     │
│ [全部] [未开始] [游玩中] [已完成]    │
└─────────────────────────────────────┘
```

下面：

```text
最近游玩
最近添加
本地位置
我的 Collection
```

---

# 34. 不要和正版游戏平台产生冲突

用户已经有：

```text
Steam
Epic
Xbox
其他启动器
```

等正版/联网游戏管理方式。

Looma 不需要取代这些平台。

Looma 的价值是：

> **统一记录“我的全部游戏资产”。**

因此：

```text
Steam 游戏
```

也可以存在：

```text
Work
type = game
```

只是：

```text
ExternalReference
→ Steam
```

而：

```text
本地单机游戏
```

可能只有：

```text
Work
+
Local Asset
```

二者可以共存。

---

# 35. 统一游戏库

最终：

```text
我的游戏库
│
├── Steam / 正版游戏
│   └── Work + ExternalReference
│
├── 本地单机游戏
│   └── Work + Local Asset
│
├── 已通关
│
├── 正在玩
│
└── 想玩
```

**不要因为来源不同而创建不同的数据模型。**

---

# 36. P0

本补丁必须完成：

```text
P0
├── Game Library View
├── Work(type=game) 统一展示
├── Game ↔ Local Asset
├── Local Directory 关联
├── Status Filter
├── Collection Filter
└── Game Detail 本地位置展示
```

---

# 37. P1

建议完成：

```text
P1
├── Game Candidate Detection
├── Duplicate / Fragment Detection
├── 多本地位置
├── 快速打开目录
├── Cover Asset
└── Game Library Statistics
```

---

# 38. P2

暂不实现：

```text
P2
├── 自动启动游戏
├── Steam API
├── Epic API
├── Playtime Tracking
├── Achievement Tracking
├── Save Game Management
├── Emulator Integration
└── Automatic Online Metadata Import
```

---

# 39. 明确禁止范围

本扩展不实现：

```text
游戏下载
游戏来源聚合
破解来源
破解补丁
破解工具
下载站索引
第三方分发
```

Looma 只管理：

```text
已经存在于用户本地的数字资产
```

---

# 40. 最终设计原则

游戏只是：

```text
Work
type = game
```

本地文件只是：

```text
Asset
```

网页只是：

```text
ExternalReference
```

进度只是：

```text
Progress
```

收藏只是：

```text
Collection
```

心得只是：

```text
Memory
```

时间记录只是：

```text
Timeline
```

因此：

```text
                   Game Work
                       │
       ┌───────────────┼────────────────┐
       ↓               ↓                ↓
    Progress         Assets          References
       │               │                │
       │          Local Directory       Web
       │
       ├── Memory
       ├── Collection
       └── Timeline
```

**不要因为增加 Game Library 就破坏 Looma 当前已经形成的统一 Domain。**

---

# 41. 最终验收

完成后，用以下真实场景验证：

### Case A：只有本地文件

```text
E:\Games\GameA\
```

结果：

```text
Game Work
+
Local Asset
```

---

### Case B：只有网页记录

```text
Game Work
+
Steam / Official Website Reference
```

没有本地 Asset 也必须合法。

---

### Case C：本地 + 网页

```text
Game Work
├── Local Asset
├── ExternalReference
├── Progress
└── Collection
```

---

### Case D：同一个游戏多个位置

```text
Game Work
├── D:\Games\GameA
├── E:\Archive\GameA
└── F:\Backup\GameA
```

必须能够关联到同一个 Work。

---

### Case E：Scanner 发现未知游戏目录

```text
Scanner
 ↓
Candidate
 ↓
User Confirm
 ↓
Create / Link Work
```

不能自动污染数据库。

---

# 42. Agent 执行要求

开始修改前：

```text
1. Inspect current Work model
2. Inspect Asset model
3. Inspect Scanner
4. Inspect ExternalReference
5. Inspect Collection
6. Inspect current Media View
7. Inspect current Progress implementation
```

然后优先：

```text
P0
```

完成后再考虑：

```text
P1
```

不要提前实现：

```text
P2
```

最终报告必须说明：

```text
Implemented
Reused Existing Architecture
New Schema / Migration
UI Changes
Scanner Changes
CLI Changes
MCP Changes
Tests
Known Limitations
```

---

# 43. 最终目标

Looma 打开后，我应该能够看到：

```text
我的数字世界
│
├── 📁 数字资产
├── 🎬 作品
├── 🎮 游戏库
├── 🧩 实体概念
├── 📚 记忆手记
├── 📑 集合收藏
└── 🕒 时间线
```

其中：

```text
🎮 游戏库
```

**不是另一个数据库。**

它只是 Looma 对：

```text
Work(type = game)
+
Local Asset
+
Progress
+
Collection
+
ExternalReference
```

进行的一层专门视图。

这使得 Looma 可以自然回答：

> **“我有什么游戏？”**

> **“它们在哪个硬盘？”**

> **“哪些还没玩？”**

> **“哪些已经通关？”**

> **“哪些游戏只有本地文件但还没有建立档案？”**

> **“这个游戏我之前记录过什么？”**

而不需要破坏 Looma 当前的统一架构。