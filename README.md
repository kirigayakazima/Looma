# Looma — Local-First Personal Digital Vault

<div align="center">

**本地优先 · 用户自主掌控 · 引用模式管理 · 可扩展的个人数字资产与记忆织机**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Cross--Platform-informational.svg)]()
[![Stack](https://img.shields.io/badge/Stack-Rust%20%2B%20Tauri%202%20%2B%20React-blueviolet.svg)]()
[![Storage](https://img.shields.io/badge/Storage-SQLite%20(WAL%20%2B%20FTS5)-success.svg)]()

[English](#english) | [简体中文](#简体中文)

</div>

---

## 简体中文

### 0. 什么是 Looma？

**Looma 是一个本地优先、用户掌控、高度可扩展的个人数字资产库与时光织机。**

它帮助用户把散落在电脑各处的个人数字资产（文件、图片、视频、项目、手记等）有机组织起来，同时**坚决不剥夺用户对自己文件的控制权**。

Looma 的核心架构思想：
> **Looma is a data system first and an application second.**  
> Looma 首先是一个本地数据系统，其次才是一个桌面应用。GUI、CLI、以及未来的 MCP (Model Context Protocol) 均为同等平级的交互客户端，统一调用底层的 Looma Core 业务逻辑。

---

### 1. 核心设计哲学

1. **本地优先（Local-First）**：
   - 100% 离线可用，不强制绑定账号、不依赖外部云端或商业闭源服务。
2. **数据属于用户（User-Owned Data）**：
   - 默认采用**引用模式（Reference Mode）**，用户原有的文件路径与存储位置保持原样不变，Looma 仅记录索引元数据与关联关系，绝不私自复制或搬移您的原盘文件。
3. **架构解耦与开放接口**：
   - AI 是外部消费者而非核心本体。通过标准协议（MCP）向外部 AI Agent 提供安全、只读/受控的资产接口，不内置强买强卖的封闭 AI 订阅。
4. **隐私安全基准**：
   - 零数据遥测、零网络追踪、无私自上传。

---

### 2. 工程架构 (Monorepo)

本项目采用严格分层的 Monorepo 架构设计：

```text
looma/
├── apps/
│   └── desktop/               # 桌面端应用客户端 (Tauri 2 + React + TypeScript + Tailwind)
│       ├── src/               # React 界面层 (多语言 I18N 默认中文，包含概览、资产、实体、记忆、收藏、设置)
│       └── src-tauri/         # Tauri 2 原生宿主 (IPC 命令路由、窗口与生命周期管理)
│
├── crates/
│   ├── core/                  # looma-core: 领域基础模型 (Asset, Entity, Relation, Memory, Collection) 与服务抽象
│   └── database/              # looma-database: SQLite 存储层 (WAL 高性能模式、FTS5 全文索引、版本化自动迁移)
│
├── docs/                      # 架构设计基线文档 (Looma Architecture Foundation)
├── package.json               # pnpm workspace 根配置文件
├── pnpm-workspace.yaml        # 前端包工作区
└── Cargo.toml                 # Cargo workspace 根配置文件
```

---

### 3. 项目开发路线图

- [x] **Phase 0 — 基础底座 (Foundation)**：Rust Workspace、Tauri 2 容器、React 桌面界面、SQLite 迁移底座、全局 I18N (默认简体中文)。
- [ ] **Phase 1 — 资产子系统 (Assets)**：增量文件系统扫描器 (`crates/scanner`)、本地元数据提取与 SHA256 哈希计算、资产浏览器。
- [ ] **Phase 2 — 实体与关系 (Entities & Relations)**：自定义通用实体、动态 JSON 属性、资产与实体双向关系网络、集合收藏 (Collections)。
- [ ] **Phase 3 — 记忆手记与时光轴 (Memories & Timeline)**：本地 Markdown 手记记录与编辑器、时序统一的时光轴 (Timeline)、人生切片快照。
- [ ] **Phase 4 — 备份与数据便携 (Backup & Portability)**：一键本地数据导出/恢复归档包 (Manifest 清单)、云端适配器抽象。
- [ ] **Phase 5 — 开放外部接口 (CLI & MCP Server)**：终端 CLI 工具、标准 MCP Server 适配，让外部 AI 安全探索与协助整理。

---

### 4. 快速开始与构建指南

#### 环境准备
- **Node.js**: >= 18.0 (推荐 v20+)
- **pnpm**: >= 9.0
- **Rust**: >= 1.80 (支持 `x86_64-pc-windows-msvc`)

#### 安装依赖
```bash
pnpm install
```

#### 启动开发环境
```bash
# 启动 Tauri 桌面原生窗口联调 (带热重载)
pnpm tauri:dev

# 或者单独在浏览器中预览界面
pnpm dev
```

#### 测试与检查
```bash
# 运行全部 Rust 单元与集成测试
cargo test --workspace

# 静态类型与编译检查
cargo check --workspace
```

#### 生产打包构建
```bash
# 构建内嵌完整离线 UI 的独立原生可执行文件
pnpm --filter @looma/desktop tauri build --no-bundle
```
构建产物将输出在 `target/release/looma-desktop.exe`。

---

## English

Looma is a local-first, user-owned, extensible personal digital vault that organizes files, entities, memories, and timelines without requiring you to surrender ownership of your files.

- **Reference Mode**: Weaves assets without duplicating or moving your files.
- **Offline & Private**: Zero telemetry, no cloud dependency, completely offline.
- **Open Standards**: Architecture decoupling GUI, CLI, and future MCP (Model Context Protocol) AI interfaces over a unified Rust Core.

---

## License

This project is licensed under the [MIT License](LICENSE).
