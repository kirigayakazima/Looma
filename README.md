# Looma — Local-First Personal Digital Vault

<div align="center">

**本地优先 · 用户自主掌控 · 引用模式管理 · 个人数字资产、作品档案与记忆织机**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Cross--Platform-informational.svg)]()
[![Stack](https://img.shields.io/badge/Stack-Rust%202021%20%2B%20Tauri%202%20%2B%20React-blueviolet.svg)]()
[![Storage](https://img.shields.io/badge/Storage-SQLite%20(WAL%20%2B%20FTS5)-success.svg)]()
[![Protocol](https://img.shields.io/badge/AI%20Protocol-Model%20Context%20Protocol%20(MCP)-orange.svg)]()

[English](#english) | [简体中文](#简体中文)

</div>

---

## 简体中文

### 0. 什么是 Looma？

**Looma 是一个本地优先、用户拥有全部数据控制权的个人数字操作系统与时光织机。**

在当今数字化生活中，我们与数字世界产生了大量的交集：散落在各块硬盘里的高清壁纸与海报、追过的番剧与漫画、通关的单机游戏、阅读的技术书籍、随手记录的观后感与评测、以及自己开发的个人项目。

Looma 的目标是帮用户把这些内容有机地编织成一个整体，同时**坚决不剥夺用户对自己文件的控制权**：
* **绝不搬移文件**：采用引用模式（Reference Mode），您的文件在原盘保持原样；
* **绝无中心化绑定**：100% 本地离线运行，零遥测、零网络追踪；
* **全端统一底座**：GUI 桌面端、终端 CLI 以及 AI MCP 服务端平级交互，统一调用 Rust 原生 `LoomaCore`。

---

### 1. 核心系统特性 (Feature Highlights)

#### 📺 作品档案与媒体追踪 (Personal Works & Media Tracking)
* **结构化文化资产管理**：支持动画/番剧 (`anime`)、漫画 (`manga`)、游戏 (`game`)、影视剧集 (`movie`/`tv_series`)、书籍小说 (`book`/`novel`) 与音乐 (`music`)。
* **即时消费进度微调 (Progress Stepper)**：
  * 精确追踪看到第几集、第几话、第几页或主线进度百分比；
  * 卡片自带 `[ - ]` / `[ + ]` 步进器，每看完一集轻点一下即时持久化；
* **外部在线入口 (External References)**：
  * 为作品聚合 Bilibili 追番页、Bangumi 条目、Steam 商店页、官方网站等入口；
  * **一键直达**：点击任意外部链接，自动呼出系统默认浏览器直接打开网页；
* **全维度数字足迹聚合**：单个作品同时穿透展示：
  * 本地海报与截图资产 (Assets)
  * 在线网页入口 (References)
  * 心得与评测手记 (Memories)
  * 所属清单与追番单 (Collections)
  * 创作者与改编系列图谱 (Relations)

#### 📁 本地资产与增量扫描器 (Assets & Incremental Scanner)
* **引用模式（Reference Mode）**：仅记录索引元数据与 SHA256 哈希，绝不私自搬移、重命名或上传您的原盘原始媒体；
* **高性能增量扫描器 (`crates/scanner`)**：基于文件修改时间与哈希指纹快速扫描，支持断点扫描与变动检测；
* **全媒体资产浏览器**：支持图片、视频、音频、文档、代码、压缩包、3D模型的多维过滤与分类预览，支持一键在系统文件管理器中定位原生文件。

#### 🧩 实体概念与通用关系网络 (Entities & Graph Relations)
* **灵活的多类型实体**：除文化作品外，支持自由登记项目 (`project`)、创作者与人物 (`person`)、地点与设备；
* **动态 JSON 属性**：支持自由扩展自定义键值元数据；
* **语义化图谱词汇表**：标准定义 `created_by`、`attaches`、`referenced_by`、`adaptation_of`、`sequel_of` 等关系，构筑个人知识网络。

#### 📖 记忆手记与统一时光轴 (Memories & Timeline)
* **本地 Markdown 手记编辑器**：内置专业 Markdown 渲染引擎，随时书写观后感、通关评测或开发随笔；
* **时序统一的时光流 (Timeline)**：将文件的被索引、作品进度的推进、手记的撰写按时间统一交织呈现。

#### 🛡️ 审计日志与自愈系统 (Audit Trail & Vault Doctor)
* **全生命周期安全审计 (Audit Trail)**：无论通过桌面端、命令行还是外部 AI MCP 调用的写操作，均自动生成防篡改审计记录；
* **数字保险库医生 (Vault Doctor)**：一键执行 SQLite 完整性检查、孤立文件扫描、失效索引智能清理与数据库压缩 (VACUUM)。

#### 📦 数据便携性与安全备份 (Data Portability & Backup)
* **Vault Manifest**：自动生成描述全部资产指纹、实体结构、集合与环境信息的标准 JSON 清单；
* **无损导出与恢复**：一键生成全量结构备份包，即便更换设备也能秒级完整还原。

#### 🤖 开放接口与 AI 协作 (CLI & Model Context Protocol)
* **全功能命令行 (`looma.exe`)**：终端直达全部核心能力（扫描、作品管理、进度更新、时间线流、备份自愈）；
* **安全 MCP 服务端 (`looma-mcp.exe`)**：标准支持 Model Context Protocol，支持 Claude Desktop、Cursor、Antigravity 等主流 AI 宿主无缝接入：
  * 暴露 `looma://works`、`looma://work/{id}`、`looma://stats`、`looma://timeline` 等统一上下文资源；
  * 提供 4 级安全权限防护模型（`ReadOnly` / `MetadataWrite` / `ContentWrite` / `FullDestructive`）。

#### 💡 确定性智能洞察 (Smart Insights)
* 内置轻量启发式规则引擎，基于目录亲和性、时间聚类与标签关联，自动化生成关系建议与集合聚合提议。

---

### 2. 工程架构 (Monorepo)

本项目采用严格分层的 Rust + Tauri + React 架构设计：

```text
looma/
├── apps/
│   └── desktop/               # 桌面端图形客户端 (Tauri 2 + React + TypeScript + Tailwind)
│       ├── src/               # React 界面层 (作品追踪、资产浏览、实体网络、手记、时光轴、洞察、设置)
│       └── src-tauri/         # Tauri 2 原生宿主 (IPC 命令分发、系统浏览器调用、原生窗口管理)
│
├── crates/
│   ├── core/                  # looma-core: 统一业务层 (LoomaCore 门面、领域模型、事件总线、服务抽象)
│   ├── database/              # looma-database: SQLite 存储引擎 (WAL 高性能模式、自动迁移、审计日志、外部引用)
│   ├── scanner/               # looma-scanner: 本地增量文件扫描器与 SHA256 哈希计算引擎
│   ├── intelligence/          # looma-intelligence: 确定性启发式关系推断与智能建议引擎
│   ├── mcp/                   # looma-mcp: 现代化 MCP 服务端与 4 级权限受控访问网关
│   └── cli/                   # looma-cli: 独立终端命令行工具 (looma 命令)
│
├── package.json               # pnpm workspace 根配置文件
├── pnpm-workspace.yaml        # 前端包工作区
└── Cargo.toml                 # Cargo workspace 根配置文件
```

---

### 3. 快速上手

#### 运行桌面版 (GUI)
已打包的可执行文件位于 `target/release/`：
* 直接双击运行：
  ```
  target/release/looma-desktop.exe
  ```
* 首次启动将自动在用户应用目录下创建并就绪您的个人数字保险库。

#### 使用命令行 (CLI)
```powershell
# 登记一部追更中的番剧
.\target\release\looma.exe work create "BLEACH 千年血战篇" --kind anime --status in_progress --orig "BLEACH 千年血戦篇"

# 更新观看进度至第 12 集
.\target\release\looma.exe work progress work_2e1ca2d1 12 --total 24 --unit 集

# 查看个人文化足迹列表
.\target\release\looma.exe work list

# 扫描并索引指定本地媒体目录
.\target\release\looma.exe scan "D:\Media\Wallpapers"
```

#### 配置 AI MCP 工具 (Claude Desktop / Cursor / Antigravity)
在您 AI 工具的 MCP 配置文件中加入：
```json
{
  "mcpServers": {
    "looma": {
      "command": "D:\\CodePackage\\Persoanl\\LoomaProject\\target\\release\\looma-mcp.exe",
      "args": ["--vault", "D:\\path\\to\\your\\vault"]
    }
  }
}
```
配置完成后，AI 即可直接理解您的数字资产库，回答诸如“我 BLEACH 看到第几集了”、“我最近添加了哪些游戏壁纸”等问题。

---

### 4. 源码构建指南

#### 环境要求
- **Node.js**: >= 18.0 (推荐 v20+)
- **pnpm**: >= 9.0
- **Rust**: >= 1.80 (`stable`)

#### 编译全部产物
```bash
# 1. 安装前端依赖
pnpm install

# 2. 运行自动化测试套件
cargo test --workspace

# 3. 构建发布版本 (CLI, MCP, Desktop)
pnpm --prefix apps/desktop tauri build --no-bundle
cargo build --release -p looma-cli -p looma-mcp
```
构建产物将输出在 `target/release/`：
* `looma-desktop.exe` (~13 MB，单文件直接运行)
* `looma.exe` (~4 MB，独立 CLI 终端工具)
* `looma-mcp.exe` (~3.2 MB，标准 MCP 服务端)

---

## English

Looma is a local-first, user-owned, extensible personal digital OS and media weaving vault that organizes your personal files, cultural works, memories, and timelines without requiring you to surrender ownership of your files.

- **Personal Media Tracking**: Structural tracking for Anime, Manga, Games, Books, Movies & Music with interactive progress steppers and direct system browser openers.
- **Reference Mode**: Weaves assets without duplicating, relocating, or altering your existing disk files.
- **Offline & Private**: Zero telemetry, no cloud dependency, completely offline.
- **Unified Core & Open Standards**: Decoupled GUI, CLI, and standard Model Context Protocol (MCP) AI interfaces over a unified Rust `LoomaCore` engine.
- **Safe & Auditable**: Built-in 4-tier MCP permission boundary, complete operation audit trails, and one-click Vault Doctor self-healing.

---

## License

This project is licensed under the [MIT License](LICENSE).
