# Looma v0.6+ 游戏封面与视觉素材架构咨询与设计方案（发给 GPT 评审）

> **文档目的**：针对个人数字资产管理软件 **Looma** 的游戏库“缺少封面图片、界面干巴”的痛点，系统梳理游戏封面/海报/图标的拉取与关联方案，评估传统元数据 API、Nintendo 官方 CDN、本地提取与 AI 的边界，供外部架构师（GPT）进行技术方案审查与建议。  
> **基准版本**：Looma v0.6 (已实现 Switch NX Activity Log 导入、PC 游戏档案与通关足迹)  
> **核心定位**：单人、本地优先、保护原文件、离线可用的 Windows 桌面 Vault。

---

## 一、用户痛点与现状分析

### 1. 现状
在 Looma v0.6 中，用户导入 Switch 游玩记录（通过 Title ID 与游戏名）或手动录入本地 PC 游戏后，游戏列表只有文字标题、游玩时长和状态徽标（游玩中/已通关/想玩）。
- **感官缺失**：缺少像 Steam、Playnite、GOG Galaxy 或 Plex/Emby 那样吸睛的竖版海报（600×900）、方块图标（Switch 512×512）或背景横幅。
- **孤立文本**：纯文字列表缺乏视觉辨识度，大量游戏堆积时难以一眼定位。
- **现有机制**：Looma 的 `Work` 实体定义中预留了 `cover_asset_id: Option<String>`，且底层已具备高效的 WebP 异步本地缩略图缓存池（Bounded Semaphore + Mtime Invalidation），但**目前缺乏“自动/半自动为游戏获取并绑定封面”的管道**。

### 2. 核心疑问
1. 获取游戏封面有哪些途径？各有什么优劣和依赖？
2. 是否需要借助 AI？AI 在其中能扮演什么角色，哪些地方不适合用 AI？
3. 下载下来的图片应该如何存储在 Looma Vault 中？（是成为标准 `Asset` 还是内置缓存？）
4. 怎样兼顾“零配置/免注册开箱即用”与“高精美度海报”？

---

## 二、潜在方案矩阵与对比评估

目前业界（如 Playnite、Steam ROM Manager、RetroArch、Jellyfin 等）成熟的方案以及 AI 方案对比如下：

| 方案类别 | 核心代表 / 机制 | 优点 | 缺点 / 门槛 | 适用场景 |
|---|---|---|---|---|
| **A. 游戏专用海报库 (SteamGridDB)** | [SteamGridDB API](https://www.steamgriddb.com/api/v2) | 专为游戏库设计，质量极高；拥有标准竖版海报 (600×900)、横版网格 (460×215)、横幅 Hero (1920×620)、透明 Logo | 需要用户申请免费 API Key；中文游戏名有时需匹配英文别名 | 高清海报首选，Playnite 等开源工具的事实标准 |
| **B. Switch 官方/社区直连 (Title ID)** | 1. Nintendo eShop CDN<br>2. 开源 TitleDB (如 hactoolnet/blawar 社区镜像) | **免 API Key**；直接根据 Switch 16 位 Title ID (如 `01007ef00011e000`) 精准匹配官方 512×512 方形图标，零误判 | 仅限 Switch 平台游戏；只有方形图标/横幅，缺少 Steam 风格的 3:2 竖版封面 | Switch 导入时的零配置即时封面 |
| **C. Steam 官方免费接口 (免 Key)** | 1. Steam Store Search 接口<br>2. Steam CDN 规则直链 (`/steam/apps/{appid}/library_600x900.jpg`) | **免 API Key**，完全公开；竖版海报极精美；Steam 拥有 PC 游戏最大覆盖率 | 非 Steam 独占/主机独占游戏搜不到；根据名称搜索存在轻微同名误差 | PC 游戏及多平台游戏的通用免 Key 方案 |
| **D. 泛 ACG 资料库 API (Bangumi / VNDB)** | [Bangumi API](https://bangumi.github.io/api/) / VNDB HTTP API | **中日文支持极佳**；番剧、Galgame、日系 RPG 覆盖全面；公开 API，免繁琐授权 | 欧美 3A 游戏覆盖较弱；图片比例多为原版包装盒，尺寸比例不统一 | 二次元、Galgame、日系单机游戏封面 |
| **E. 本地启发式挖掘 (Local Heuristic)** | 1. 本地目录搜索 `cover.jpg` / `poster.png` / `folder.jpg`<br>2. 从 Windows `game.exe` 提取内嵌高清图标 (`.ico` -> PNG) | **完全离线**；不依赖外部网络；直接呈现用户本地定制的图片 | 很多游戏安装包并不自带标准封面；Switch 游戏完全没有本地文件可挖 | PC 扫描关联时的本地底牌 |
| **F. AI 检索与生成 (LLM / VLM / SD)** | 1. 搜索引擎图片爬取（Google/Bing Custom Search）<br>2. 大模型生成封面（Midjourney/DALL-E/Flux） | 任何冷门/独立甚至自制游戏都能“凭空造图” | **严重不推荐生成**（文字乱码、封面变形、成本极高、速度慢）；网页爬虫质量不可控且容易被封 IP | 仅适合作为“别名模糊对齐”（见下文） |

---

## 三、AI 在此场景中的定位探讨：何时需要，何时坚决不要？

针对用户的疑问：**“还是说要借助 AI？”**，我们的初步研判如下：

### 1. 坚决不要用 AI 的地方：
- **不要用 AI 生图模型（如 DALL-E / Flux）生成封面**：
  游戏封面具有高度权威性（官方海报、特定排版、原作 Logo、画师署名）。AI 生成的封面不仅成本昂贵、耗时数秒，且 Logo 字符常有乱码变形，完全无法替代官方海报的质感。
- **不要用 AI 代理（Tool Calling Agent）自动去公网到处搜图下载**：
  不可控的网页爬虫容易抓回带水印、壁纸分辨率不合规、甚至同人二次创作的低清图，且容易导致网络封禁。

### 2. AI 真正有价值的轻量级辅助角色（可选增强）：
- **跨语种别名对齐（Fuzzy Title Translation & Normalization）**：
  - 例如用户导入 Switch 游戏，JSON 中的名称是中文“**塞尔达传说 旷野之息**”或日文“**ゼルダの伝説 ブレス オブ ザ ワイルド**”。
  - 当调用 SteamGridDB 或 Steam API 时，海外数据库往往以英文“**The Legend of Zelda: Breath of the Wild**”作为主索引。
  - **AI 助手（本地轻量规则或单轮短文本 LLM）**：只需输入中文名，输出标准英文国际通用名与候选别名（格式为纯 JSON），将查询命中率由 60% 提高至 98%，随后再由 Rust 后端向 SteamGridDB 发起精准请求。

---

## 四、建议的分层架构设计草案（Looma Cover Pipeline）

为兼顾**“零配置上手”、“极致美观”与“本地 Vault 离线优先”**，提出如下三层混合流水线：

```text
[用户触发] 
  - Switch 导入时自动拉取
  - 或在游戏卡片/抽屉点击“获取/更换封面”
         │
         ▼
[Phase 1: 零配置即时解析 (Zero-Config Layer)]
  ├─ Switch 游戏: 利用 16 位 Title ID，直接从 Nintendo CDN/社区 TitleDB 获取 512x512 官方图标
  └─ PC 游戏: 目录若有 cover.png/jpg 直接优先使用；若无则通过 Steam Store Search 免 Key 获取 600x900 竖版海报
         │
         ▼ (若未命中或用户希望挑选更高清海报)
[Phase 2: 高清网格与社区库 (SteamGridDB / Bangumi)]
  ├─ 可选配置：用户可填入一个免费的 SteamGridDB API Key
  ├─ 弹出“封面选择器 (Cover Picker Modal)”：展示 4~8 张不同画风的候选海报（竖版 600x900、横版网格、极简插画风）
  └─ 用户点击一张满意的封面
         │
         ▼
[Phase 3: Vault 持久化与本地离线化]
  ├─ 将选中的图片下载并以 SHA-256 存入 Vault 内部资源目录（例如 `.looma/media/covers/{hash}.webp`）
  ├─ 创建或复用一个 Asset 记录（kind = "image"），并将该 Asset ID 写入 Work 的 `cover_asset_id`
  └─ 利用已有的 Thumbnail Cache 生成 256px WebP 缩略图，网格瞬间流畅渲染
```

---

## 五、向 GPT 咨询的 5 个关键架构问题

请 GPT 结合 Playnite、Calibre、Plex 等成熟本地媒体库的演进经验，对以下 5 个决策点给出明确建议与最佳实践：

### 1. 外部数据源的选型与优先级策略
> **问题**：针对单人多平台（Switch + Steam + PC 单机 + 模拟器），最佳的默认数据源链条应该如何排序？是否应该将“SteamGridDB (需要 Key)”作为高级选项，而将“Nintendo TitleID 官方 CDN + Steam Search (免 Key)”作为默认开箱即用层？有没有其他免 Key 的高质量游戏封面源推荐？

### 2. Switch Title ID 的官方直连图源可用性
> **问题**：Switch 的 16 位 Title ID（如 `01007ef00011e000`）目前有哪些公开稳定、无需反代/翻墙即可访问的官方或社区 CDN 图片链接结构？对于中国大陆或不同区域网络，是否存在访问受阻风险？是否有成熟可靠的公开镜像（如 GitHub 托管的 TitleDB icons）？

### 3. 下载图片的存储位置与 Asset 领域模型
> **问题**：在 Looma 的设计原则中，“保护用户原始文件不被污染”是最高红线。
> 下载的封面图片应该：
> - **A 方案**：直接作为标准的 `Asset`（如同扫描进去的文件一样，分配 Asset UUID，存入 Vault 内部的媒体文件夹），从而天然复用 Asset 缩略图、Hash 去重和全量备份恢复机制？
> - **B 方案**：作为 Work 的私有缓存附件，不作为独立 Asset，仅存放在 `.looma/covers/{work_id}.webp`？  
> 哪种方案在长期备份恢复、作品删除级联和性能上更合理？

### 4. 自动化下载 vs 手动交互选择的平衡
> **问题**：在导入 50 款游戏时，是应该：
> - **全自动后台静默下载第一张匹配图**（可能偶尔有误匹配或比例不协调）；
> - 还是**只做半自动**（导入后显示通用游戏占位卡，用户在详情抽屉点击“挑选封面”，弹窗呈现多张候选图由用户一键选定）；
> - 抑或是**默认静默匹配 + 随时支持重新挑选**？在桌面端应用中，哪种体验最舒服？

### 5. AI 在元数据与海报获取中的真正价值
> **问题**：如果引入本地单轮文本 LLM 或搜索增强，AI 是否值得做更多事情？比如：
> - 自动分析游戏目录中的 EXE 名称（如 `Hades.exe` -> 《哈迪斯 / Hades》）？
> - 提取游戏类型、开发商、发售年份？  
> 还是说在 v0.6+ 阶段，我们应该严格将 AI 限制在“别名/翻译规范化”，不要过度神化 AI 在图片拉取中的作用？

---

## 六、结语与期望输出格式

期望 GPT 的回复包含：
1. **直接结论**：对上述 5 个问题的逐项明确裁定。
2. **轻量实施路线（Phase 0 -> Phase 1）**：给出最具性价比、代码量最小但效果立竿见影的极简落地步骤。
3. **避坑警示**：过去类似开源项目（如 Playnite、Kodi、RetroArch）在封面刮削上踩过的最大典型坑（如封 IP、图片尺寸参差不齐、死链等）及预防手段。
