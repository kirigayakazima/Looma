# Looma Generic Work Type Model Design (v0.1)

> **文档版本**: v0.1  
> **状态**: **ARCHITECTURE DESIGN DRAFT**  
> **前序基准**: `Looma Game Library v0.1 — Freeze Acceptance & Generic Work Expansion Guardrails.md`  
> **执行原则**: **设计先行、架构统一、严禁为每个媒介类型建立平行割裂系统**  
> **回归基准**: `Game Library v0.1` 既有模型、API 与 8 项自动化回归测试为不可退化基线  

---

## 1. 愿景与设计哲学

Looma 的终极定位是一个**本地优先的个人数字人生操作系统 (Local-First Personal Digital OS)**。  
在现实生活中，个人的精神文化与创作记录是丰富且多元的：我们游玩单机游戏、追看季度新番、翻阅连载漫画、精读专业书籍、鉴赏经典电影、收听音乐专辑、推进个人工程项目。

### 1.1 核心误区：七套平行的割裂系统
传统软件常犯的架构错误，是针对每一个类型建立专门的应用或专门的子模块：
```text
❌ 传统割裂做法：
Looma
 ├── Game Manager   (GameDB, GameAsset, GameProgress, GameUI)
 ├── Anime Tracker  (AnimeDB, AnimeAsset, AnimeProgress, AnimeUI)
 ├── Manga Reader   (MangaDB, MangaAsset, MangaProgress, MangaUI)
 ├── Book Shelf     (BookDB, BookAsset, BookProgress, BookUI)
 └── Movie Archive  (MovieDB, MovieAsset, MovieProgress, MovieUI)
```
这种做法会导致模型严重膨胀、数据库迁移频繁断裂、代码大量重复复制，并且无法表达跨媒介关系（例如：“这部《攻壳机动队》电影改编自同名漫画，且附带原声集和本地游戏”）。

### 1.2 Looma 的核心准则：统一作品系统 (Generic Work System)
```text
✔ Looma 统一架构：
                     Looma Core
                         │
              ┌──────────▼──────────┐
              │  Generic Work Model │
              └──────────┬──────────┘
                         │
         ┌───────────┬───┴───────┬───────────┐
         ▼           ▼           ▼           ▼
       Game        Anime       Manga       Book...
         │           │           │           │
         └───────────┼───────────┴───────────┘
                     │
         Generic Shared Capabilities
         (100% 共享统一底层底座)
                     │
       ┌─────────────┼─────────────┬─────────────┐
       ▼             ▼             ▼             ▼
     Asset        Relation      Progress      Memory
  (文件/目录)    (双向图网)    (步进/状态)   (反思手记)
                     │
         ┌───────────┴───────────┐
         ▼                       ▼
     Collection              Reference
     (收藏切片)            (Steam/BGM/Web)
```
- **一个作品 (Work)** 就是一个通用的核心实体；
- **类型 (Type)** 只是它的一个属性和元数据视口；
- **所有通用能力（资产管理、进度追踪、关系网络、手记、收藏夹、外部引用、时光轴）全量共享**；
- 只有真正具有类型特异性的字段（如集数、卷数、分级、平台），才作为类型特异元数据（Type-specific Metadata）叠加在通用模型上。

---

## 2. 作品类型定义 (WorkType Taxonomy)

在 `crates/core/src/models/work.rs` 中，`WorkType` 采用开放且完备的枚举进行统一表达：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkType {
    // 电子游戏 (已在 v0.1 验证通过)
    Game,
    // 动画与番剧
    Anime,
    // 漫画与图集
    Manga,
    // 图书、小说与出版物
    Book,
    Novel,
    // 电影与短片
    Movie,
    // 电视剧与连续剧
    TvSeries,
    // 音乐、单曲与专辑
    Music,
    Album,
    // 播客与音频节目
    Podcast,
    // 纪录片与公开课
    Documentary,
    // 个人创作与工程项目
    Project,
    // 自定义或未知类型
    Other,
}
```

### 2.1 分类范畴归并
为了便于 UI 归类与用户心智认知，所有类型可划分为三大群组：
1. **Interactive & Software (交互与软件)**: `Game`, `Project`
2. **Episodic & Streaming (时序与连续流)**: `Anime`, `TvSeries`, `Movie`, `Documentary`, `Podcast`
3. **Print & Literature (阅读与文献)**: `Manga`, `Book`, `Novel`
4. **Audio & Musical (声学与音乐)**: `Music`, `Album`

---

## 3. 通用能力底座 (Generic Capabilities Matrix)

无论一个 `Work` 是何种类型，它都**无条件拥有**以下标准能力：

| 通用能力 | 领域模型 | 存储位置 | 语义解释 |
| :--- | :--- | :--- | :--- |
| **基础档案** | `Entity` | `entities` | 唯一 ID、标题、原始语言名称、描述/简介、创建与更新时间 |
| **消费状态** | `RecordStatus` | `entities.properties_json.work.status` | Planned (想看/想玩), InProgress (进行中), Completed (已通关/已读完), Paused (搁置), Dropped (弃坑), Revisit (二刷/重温) |
| **数字资产关联** | `Asset` + `Relation` | `assets` + `relations` | 本地关联的文件夹、视频文件、PDF、EPUB、ROM、安装目录、封面图等 |
| **消费进度跟踪** | `WorkProgress` | `entities.properties_json.work.progress` | 当前位置、位置类型（集/卷/章/页/时间点/百分比）、总位置、单位 |
| **网状关系网络** | `Relation` | `relations` | 创作者、制作公司、原作改编、续作/前传、番外、原声集等双向链接 |
| **反思手记** | `Memory` | `memories` + `relations` | 个人的游玩心得、追番手记、读书笔记、评分、Markdown 归档 |
| **收藏集切片** | `Collection` | `collections` + `collection_items` | 自定义聚合（如“年度十佳”、“赛博朋克大系”、“神作榜”） |
| **外部权威参考** | `ExternalReference` | `external_references` | Bangumi、Steam、豆瓣、Goodreads、Bilibili、IMDb、官网链接 |
| **时序时光轴** | `TimelineItem` | 动态聚合视图 | 记录人生中何时开始、何时通关、何时写下反思的统一时序切片 |

---

## 4. 类型特异性元数据设计 (Type-Specific Metadata Overlays)

### 4.1 核心准则：杜绝特异性数据库建表
**严禁新增任何 `AnimeTable`、`MangaTable`、`BookTable`！**  
所有特异性元数据均安全保存在通用实体表的 `properties_json` 中，Rust 侧通过类型安全枚举与结构体反序列化。

### 4.2 统一结构：`WorkMetadata`
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkMetadata {
    pub work_type: WorkType,
    pub status: RecordStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_asset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<WorkProgress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f64>,
    
    // 类型特异性扩展元数据 (Optional Overlays)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_metadata: Option<TypeSpecificMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeSpecificMetadata {
    Game(GameSpecificMeta),
    Anime(AnimeSpecificMeta),
    Manga(MangaSpecificMeta),
    Book(BookSpecificMeta),
    Movie(MovieSpecificMeta),
    TvSeries(TvSeriesSpecificMeta),
    Music(MusicSpecificMeta),
}
```

### 4.3 各类型特异字段细则

#### (1) Anime (动画/番剧)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimeSpecificMeta {
    pub season: Option<String>,           // 例: "2024-10" (2024秋季番)
    pub total_episodes: Option<u32>,       // 总集数，例: 24
    pub anime_format: Option<String>,     // tv | movie | ova | ona | special
    pub studio: Option<String>,           // 制作公司，例: "MAPPA", "ufotable"
    pub broadcast_day: Option<String>,    // 放送星期，例: "Saturday"
}
```

#### (2) Manga (漫画)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MangaSpecificMeta {
    pub total_volumes: Option<u32>,        // 总单行本卷数
    pub total_chapters: Option<u32>,       // 总话数
    pub serialization_status: Option<String>, // ongoing (连载中) | completed (已完结)
    pub magazine: Option<String>,          // 连载期刊，例: "周刊少年JUMP"
}
```

#### (3) Game (单机游戏 — 已验证基线)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSpecificMeta {
    pub platform: Option<Vec<String>>,    // PC, PS5, Switch, SteamDeck
    pub developer: Option<String>,        // 开发商，例: "FromSoftware"
    pub publisher: Option<String>,        // 发行商，例: "Bandai Namco"
    pub engine: Option<String>,           // Unreal, Unity, Proprietary
}
```

#### (4) Book / Novel (图书与文学)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSpecificMeta {
    pub isbn: Option<String>,             // 国际标准书号 (10/13位)
    pub author: Option<String>,           // 作者
    pub translator: Option<String>,       // 译者
    pub publisher: Option<String>,        // 出版社
    pub total_pages: Option<u32>,         // 总页数
    pub edition: Option<String>,          // 版次，例: "精装第2版"
}
```

#### (5) Movie (电影与剧场版)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieSpecificMeta {
    pub runtime_minutes: Option<u32>,     // 时长 (分钟)
    pub director: Option<String>,         // 导演
    pub country: Option<String>,          // 制片国家/地区
    pub language: Option<String>,         // 原始语言
}
```

---

## 5. 通用消费进度模型 (Universal Progress Model)

不同文化媒介的步进维度千差万别，但在数学和认知上，都可抽象为：
> **`当前坐标 (Position)` / `总体容量 (Total)` + `计量单位 (Unit)`**

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkProgress {
    pub position: f64,                           // 当前步进点 (支持小数，如 12.5 集)
    pub position_type: ProgressPositionType,      // 步进维度类型
    pub total_positions: Option<f64>,            // 标称上限 (如 24 集、686 话、420 页)
    pub unit: Option<String>,                    // 呈现单位 (如 "集", "话", "卷", "页", "%")
    pub updated_at: DateTime<Utc>,               // 最近更新时间戳
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressPositionType {
    Episode,      // 适用于 Anime, TvSeries
    Chapter,      // 适用于 Manga, Book, Novel
    Volume,       // 适用于 Manga 单行本
    Page,         // 适用于 Book, Paper
    Minute,       // 适用于 Movie, Podcast
    Percentage,   // 适用于 Game, Project
    Custom(String), // 用户自由自定义单位
}
```

### 5.1 默认步进推导策略
当用户录入作品未明确指定单位时，系统根据 `WorkType` 进行无感自然推导：
- `Anime` / `TvSeries` → `Episode` (单位: `集`)
- `Manga` → `Chapter` (单位: `话`)
- `Book` / `Novel` → `Chapter` 或 `Page` (单位: `章` / `页`)
- `Movie` → `Minute` (单位: `分`)
- `Game` → `Percentage` 或 `Custom("章")`
- 任意类型均支持一键 `+1` / `-1` 快捷步进，并自动联动更新时光轴记录。

---

## 6. 通用资产映射模式 (Universal Asset Patterns)

各类作品在物理磁盘上的落盘形态各不相同，但均通过 `LoomaCore` 的两大基本资产单元表达：
1. **`AssetKind::Directory` (目录资产)**: 用于代表整个作品的聚合根；
2. **`AssetKind::File` (单文件资产)**: 用于代表具体音视频、电子书、封面图片、字幕。

| 作品类型 | 典型本地资产场景 | Asset 组织模式 |
| :--- | :--- | :--- |
| **Game** | `E:/Games/NieR_Automata` (单机根目录) | `AssetKind::Directory` (`attaches`) |
| **Anime** | `D:/Anime/BLEACH_TYBW/` (包含 S01E01-E24.mkv) | `Directory` (追番根目录) + `Video` (分集文件) + `Image` (海报) |
| **Manga** | `E:/Comics/ChainsawMan/Vol_01.cbz` 或章节目录 | `Archive` (压缩包) 或 `Directory` (卷目录) |
| **Book** | `D:/Books/Rust_Programming.pdf` / `.epub` | `Document` (`mime_type: application/pdf`) |
| **Movie** | `E:/Movies/Interstellar.2014.2160p.mkv` | `Video` (正片) + `Document` (外挂 .ass 字幕) |

所有资产与作品的绑定统一使用：
```text
Work ─── relation_type = "attaches" ───> Asset
```
**严格遵循 Game Library v0.1 审计守则**：所有被关联的资产必须真实存在于 `assets` 表中，坚决不允许制造任何脱离数据库实体的伪造 ID。

---

## 7. 网状图谱与跨媒介关系 (Cross-Media Knowledge Graph)

个人数字藏品最吸引人之处在于**知识互联**。Looma 采用通用的三元组关系表（`relations`）支持任意方向的语义联结：

```text
    [Person: 久保带人] ◄── created_by ────┐
                                         │
    [Manga: BLEACH] ◄── adaptation_of ── [Anime: BLEACH 千年血战篇]
          │                                      │
          ├── soundtrack_of ── [Music: 剧伴OST]   ├── attaches ── [Asset: 本地mkv目录]
          │                                      │
          └── has_game ────── [Game: 刀鸣]        └── has_ref ─── [Bangumi: 371234]
```

### 7.1 标准关系动词定义 (Standard Relation Types)
- `created_by` / `directed_by` / `authored_by`: 创作者归属 (Work → Person/Org)
- `published_by`: 出版或发行方 (Work → Organization)
- `adaptation_of`: 改编源流 (如 Anime → Manga, Game → Anime)
- `sequel_of` / `prequel_of`: 前后时序 (Work → Work)
- `spin_off_of`: 外传与衍生关系 (Work → Work)
- `soundtrack_of`: 原声配乐关联合并 (Music → Game/Anime/Movie)
- `attaches`: 本地数字资产挂载 (Work → Asset)
- `references`: 外部公网权威页面 (Work → ExternalReference)

---

## 8. 三端交付契约 (GUI, CLI, MCP Consistency)

三端一致性已在 Game Library v0.1 验证通过。对于后续所有 Work Type，严格保持同等调用链路：

### 8.1 GUI 呈现原则
- **不分裂多套页面系统**：提供统一的“文化作品 / 个人记录”总览，支持按 `Type` 快捷切页（全部 / 游戏 / 动漫 / 漫画 / 电影 / 图书）；
- **卡片复用机制**：网格视图统一复用卡片底座（封面、标题、状态徽章、步进器），根据类型自适应切换辅标（如游戏显示盘符，动画显示“第 12/24 集”，图书显示“第 340/500 页”）；
- **统一详情抽屉**：抽屉内模块自适应排布（进度调整、本地资产/多路径、外部链接、关联创作者、反思手记）。

### 8.2 CLI 终端命令规范
通用 `looma work` 命令天然面向全类型开放：
```bash
# 查询不同类型的作品
looma work list -t anime
looma work list -t manga
looma work list -t book

# 录入新作品
looma work create "电锯人" -t manga --status in_progress
looma work create "深入理解计算机系统" -t book --status planned

# 步进消费进度
looma work progress <ID> 12 -t episode   # 动画第12集
looma work progress <ID> 95 -t chapter   # 漫画第95话
looma work progress <ID> 350 -t page     # 图书第350页

# 挂载本地资源目录
looma work link-dir <WORK_ID> "D:/Anime/ChainsawMan"
```

### 8.3 MCP 服务端契约
无需为每个类型扩展专用工具，外部 AI 助手直接调用现有工具集即可达成全媒体管理：
- `search_entities`: 传入 `entity_type: "anime" | "manga" | "book" | "game"`；
- `create_entity`: 统一创建任意类型的 Work；
- `update_work_progress`: 统一更新步进点；
- `ensure_directory_asset` / `link_work_directory`: 统一挂载目录资产；
- 资源 `looma://works`: 暴露全局作品列表；
- 资源 `looma://work/{id}`: 读取跨媒介聚合的 `WorkSummary`。

---

## 9. 数据库兼容性与平滑演进 (Zero Breaking Migration)

### 9.1 既有 Schema 完美容纳
Looma 现有的 SQLite 数据库结构（`entities`, `assets`, `relations`, `memories`, `collections`, `external_references`）具有完备的前瞻性抽象：
- 不需要新增任何 `_anime`、`_manga` 数据库表；
- 不需要对现有表执行破坏性 `ALTER TABLE`；
- 所有已录入的单机游戏库数据**保持 100% 原样兼容**。

### 9.2 索引与性能预研
随着作品数量和资产数量上升，若需要进一步优化高频检索，仅需在未来的 Migration 中按需追加轻量索引：
```sql
-- 预研：优化跨类型与状态的高频联合检索 (未来可选，当前无需强依赖)
CREATE INDEX IF NOT EXISTS idx_entities_type_status ON entities(entity_type);
```

---

## 10. 实施里程碑与回归基线防线

为杜绝冒进并确保生产级代码质量，通用作品扩展必须按以下分阶段节奏推进：

```text
Phase A (当前完成):
  Game Library v0.1 冻结 + 最终审计验收 (10/10 PASS) + 本架构设计文档

Phase B (下一阶段):
  Core Domain 元数据结构体完善 (AnimeSpecificMeta, MangaSpecificMeta 等)
  + 单元测试覆盖各类型的序列化/反序列化

Phase C:
  GUI 统一 Works 视图适配 (支持 Anime, Manga, Book, Movie 分面筛选与专用步进呈现)

Phase D:
  三端联合测试 + Game Library v0.1 全量回归断言确认未被破坏
```

> **坚守回归底线**：  
> 任何后续针对动漫、漫画、图书的开发与重构，**必须始终运行 `crates/database/tests/work_test.rs` 中的 Game Library 回归测试集**。只要任何单机游戏、目录资产、离线探测或多盘符功能出现单点退化，该次变更立即作废。
