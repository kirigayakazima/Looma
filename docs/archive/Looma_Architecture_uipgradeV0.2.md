# Looma v0.2 — Architecture Hardening Plan

> **目标：在不推翻现有 v0.1 数据模型和 Local-First 设计的前提下，把 Looma 从“可用的个人资产库原型”提升为“可被 CLI / MCP / AI / Plugin / Cloud 长期扩展的个人数字资产底座”。**

---

# 0. 本阶段最高优先级

本阶段**不是功能大扩张**。

禁止因为看到 Gap Analysis 就继续堆：

* OCR
* Vector DB
* Semantic Search
* LLM Agent
* Plugin Marketplace
* Cloud Sync
* Mobile App
* Managed Asset 大规模实现
* 复杂 Event Bus
* 多设备实时同步

这些都不是 v0.2 的核心目标。

v0.2 的核心目标只有一句话：

> **确保未来增加 GUI、CLI、MCP、Plugin、Cloud、Automation、AI 等能力时，不需要绕过 Core 直接操作数据库，也不需要推翻现有数据模型。**

---

# 1. 当前架构判断

当前 Looma v0.1 已经具备：

* Asset
* Entity
* Relation
* Collection
* Memory
* Timeline
* Search
* Reference Asset
* Local Backup
* CLI
* 基础 MCP
* 基础 deterministic intelligence
* vault dump / Markdown / manifest 等数据导出能力

因此当前版本已经属于：

> **Looma v0.1 — Usable Local-First Personal Asset Library**

但当前仍存在几个结构性缺口：

```text
Current

Desktop ─────┐
CLI ─────────┼──→ DB
MCP ─────────┘

各入口存在直接访问 DB / Scanner / Intelligence 等内部组件的情况
```

目标架构：

```text
                    Looma Core
                        │
          ┌─────────────┼─────────────┐
          ↓             ↓             ↓
       Desktop          CLI          MCP
                                      │
                                      ↓
                              External AI / Tools

Core 内部：

LoomaCore / VaultContext
 ├── AssetService
 ├── EntityService
 ├── RelationService
 ├── MemoryService
 ├── CollectionService
 ├── SearchService
 ├── ScannerService
 ├── BackupService
 ├── ReferenceService
 └── ...
```

---

# 2. v0.2 开发优先级

## P0 — 必须完成

```text
P0
├── Core Facade / LoomaCore / VaultContext
├── MCP Resources
├── MCP Permission Boundary
└── Audit Log
```

这四项是 v0.2 的核心。

---

## P1 — 应该完成

```text
P1
├── ExternalReference
├── Data Manifest 强化
├── CloudStorage Trait
└── Domain Event Contract
```

这些项目主要用于建立未来扩展边界。

---

## P2 — 只设计边界，暂不做完整实现

```text
P2
├── Plugin Contracts
├── Managed Asset
└── Cloud Provider Adapter
```

允许建立 Trait / Interface / Contract。

暂时不要求动态 Plugin Runtime，也不要求真正接入多个 Cloud Provider。

---

## P3 — 暂不进入 v0.2

```text
P3
├── OCR
├── Semantic Search
├── Vector Database
├── LLM Agent
├── Plugin Marketplace
├── Mobile Client
└── Multi-device Sync
```

---

# 3. P0-1 — Core Facade

## 3.1 目标

建立真正的 Application/Core Facade。

核心原则：

> **UI / CLI / MCP 不应该直接操作数据库。**

禁止长期存在：

```text
MCP → SQLite
CLI → SQLite
Desktop → SQLite
```

目标：

```text
MCP ─────┐
CLI ─────┼──→ LoomaCore ──→ Repository / DB
Desktop ─┘
```

---

# 4. LoomaCore 建议结构

可以根据当前 Rust 项目实际情况调整命名。

推荐：

```text
src/
├── core/
│   ├── mod.rs
│   ├── context.rs
│   ├── asset_service.rs
│   ├── entity_service.rs
│   ├── relation_service.rs
│   ├── memory_service.rs
│   ├── collection_service.rs
│   ├── search_service.rs
│   ├── scanner_service.rs
│   ├── backup_service.rs
│   ├── reference_service.rs
│   └── ...
│
├── repository/
│   ├── ...
│
├── domain/
│   ├── asset.rs
│   ├── entity.rs
│   ├── relation.rs
│   ├── memory.rs
│   └── ...
│
├── mcp/
├── cli/
└── desktop/
```

实际目录可以不同。

关键不是目录名称，而是：

> **Core 成为唯一推荐的业务操作入口。**

---

# 5. Core API 原则

例如：

```rust
core.assets.create(...)
core.assets.get(...)
core.assets.update(...)
core.assets.delete(...)

core.entities.create(...)
core.entities.get(...)
core.entities.update(...)

core.relations.create(...)
core.relations.delete(...)

core.memories.create(...)
core.memories.update(...)

core.search.search(...)
```

而不是：

```rust
mcp → db.insert(...)
cli → db.insert(...)
desktop → db.insert(...)
```

---

# 6. Core Facade 的边界

Core 应负责：

* 业务规则
* 权限检查
* 数据校验
* Repository 调用
* Audit
* Event emission
* Manifest 更新相关逻辑
* Backup 相关业务操作

Core 不应该负责：

* MCP protocol
* CLI argument parsing
* GUI rendering
* HTTP protocol
* LLM provider
* Cloud provider 的具体实现

原则：

> **Protocol 层调用 Core，Core 不知道调用它的是 MCP、CLI 还是 Desktop。**

---

# 7. P0-2 — MCP Resources

当前已有 MCP Tools：

```text
tools/list
tools/call
```

v0.2 增加：

```text
resources/list
resources/read
```

---

# 8. MCP Resource URI

建议支持类似：

```text
looma://asset/{id}
looma://entity/{id}
looma://memory/{id}
looma://collection/{id}
looma://timeline/{id}
```

也可以根据现有数据模型调整。

核心要求：

> Resource 是“读取 Looma 内部结构化资源”的标准入口。

例如：

```text
AI
 │
 ├── search_assets
 ├── get_asset
 └── resources/read
        │
        └── looma://asset/123
```

---

# 9. MCP Tool 与 Resource 的职责区别

不要把两者混成一回事。

### Tool

用于：

```text
search
create
update
delete
scan
backup
```

### Resource

用于：

```text
读取一个稳定、结构化、可引用的 Looma Resource。
```

例如：

```text
looma://entity/bleach
```

返回：

```json
{
  "id": "...",
  "name": "BLEACH",
  "type": "anime",
  "relations": [...],
  "references": [...]
}
```

具体 schema 根据实际实现决定。

---

# 10. P0-3 — MCP Permission Boundary

这是 v0.2 必须解决的问题。

不要依赖：

> “目前 MCP 没有危险工具，所以暂时安全。”

应该建立明确权限模型。

推荐至少分成：

```text
READ
METADATA_WRITE
CONTENT_WRITE
DESTRUCTIVE_WRITE
```

例如：

```text
READ
├── search
├── get
└── resources/read

METADATA_WRITE
├── create entity
├── update entity
├── create relation
└── update metadata

CONTENT_WRITE
├── create memory
├── update memory
└── modify managed content

DESTRUCTIVE_WRITE
├── delete
├── bulk delete
├── move
└── destructive operations
```

---

# 11. MCP 默认权限

默认原则：

```text
READ                allowed
METADATA_WRITE      controlled
CONTENT_WRITE       controlled
DESTRUCTIVE_WRITE   denied by default
```

如果当前 MCP server 没有认证 / session / user identity，也不要假装有完整安全体系。

应该：

> **明确建立 capability boundary，而不是制造虚假的复杂认证系统。**

---

# 12. Destructive Operation Boundary

所有高风险操作必须经过明确的 boundary。

例如：

```text
delete_asset
delete_entity
delete_relation
bulk_delete
```

不得直接：

```text
MCP → DB DELETE
```

应该：

```text
MCP
 ↓
Permission Check
 ↓
Confirmation / Policy
 ↓
LoomaCore
 ↓
Audit
 ↓
Repository
```

---

# 13. P0-4 — Audit Log

建立最小可用 Audit。

不要求现在实现完整企业级审计系统。

但至少需要记录：

```text
timestamp
actor
operation
target_type
target_id
result
before
after
```

例如：

```json
{
  "timestamp": "...",
  "actor": "mcp",
  "operation": "entity.update",
  "target_type": "entity",
  "target_id": "...",
  "result": "success"
}
```

---

# 14. Actor

建议区分：

```text
desktop
cli
mcp
system
plugin
automation
```

未来可以扩展：

```text
cloud
sync
```

这样未来才能回答：

> “这个东西是谁改的？”

---

# 15. Audit 与 Undo

v0.2：

```text
Audit = MUST
Undo = OPTIONAL
```

不要为了实现 Undo 直接把系统改造成完整 Event Sourcing。

只要 Audit 能够记录关键 mutation 即可。

未来如果需要：

```text
Undo
Version History
Change Replay
```

可以建立在 Audit / Event 基础上进一步发展。

---

# 16. P1-1 — ExternalReference

增加独立的 ExternalReference 模型。

目的：

> Looma 不应该只管理本地文件，还应该成为个人数字资产之间的连接层。

例如：

```text
Entity
  └── ExternalReference
       ├── GitHub
       ├── Bilibili
       ├── Website
       ├── Documentation
       └── Other
```

---

# 17. ExternalReference 建议字段

例如：

```text
id
target_type
target_id
url
provider
title
description
created_at
updated_at
```

可以根据实际需求调整。

例如：

```json
{
  "provider": "github",
  "url": "https://github.com/...",
  "title": "Looma Project"
}
```

---

# 18. ExternalReference 与 Asset 的关系

不要强制：

```text
URL = Asset
```

二者应该区分。

```text
Asset
→ 本地可管理/引用的数字资产

ExternalReference
→ 外部系统中的引用
```

例如：

```text
Asset
  └── screenshot.png

ExternalReference
  └── https://github.com/...
```

二者可以通过 Relation 连接。

---

# 19. P1-2 — Data Manifest

当前已有：

```text
manifest.json
vault_dump.json
memories/*.md
```

v0.2 强化 Manifest。

至少考虑：

```text
schema_version
vault_id
created_at
updated_at
export_version
checksums
```

如果已有字段，则不要重复创建。

---

# 20. Manifest 的核心目标

Manifest 不是数据库备份本身。

它应该回答：

> “这个 Vault 到底是什么版本、包含什么、导出是否完整、文件有没有发生变化？”

例如：

```json
{
  "schema_version": "0.2",
  "vault_id": "...",
  "export_version": 1,
  "checksums": {
    "vault_dump.json": "...",
    "memories/2026-10-03.md": "..."
  }
}
```

具体格式以实际实现为准。

---

# 21. 数据恢复原则

继续保持 Local-First：

```text
SQLite
   ↓
可恢复状态

Export
   ↓
Portable Representation
```

目标：

> SQLite 丢失时，不能导致 Looma 中所有个人数据永久丢失。

---

# 22. P1-3 — CloudStorage Trait

v0.2 **不要求真正实现 Google Drive / OneDrive / S3 / WebDAV。**

先定义抽象。

例如：

```rust
trait CloudStorage {
    async fn upload(...);
    async fn download(...);
    async fn list(...);
    async fn delete(...);
    async fn exists(...);
    async fn verify(...);
}
```

实际 Trait API 可以根据 Rust 项目现状设计。

---

# 23. Cloud Adapter 原则

目标：

```text
LoomaCore
    │
    ↓
BackupPackage
    │
    ↓
CloudStorage Trait
    │
    ├── Local
    ├── GCS
    ├── S3
    ├── WebDAV
    └── Google Drive
```

具体 Provider 后续实现。

---

# 24. Cloud 不应该侵入 Domain Model

不要为了 Cloud：

```text
Asset
  ↓
增加大量 cloud-specific fields
```

更推荐：

```text
Asset
   │
   └── Backup / Sync Layer
           │
           └── CloudStorage
```

保持 Domain 独立。

---

# 25. P1-4 — Domain Event Contract

v0.2 不要求复杂 Event Bus。

先定义 Domain Event。

例如：

```text
AssetCreated
AssetUpdated
AssetDeleted

EntityCreated
EntityUpdated
EntityDeleted

RelationCreated
RelationDeleted

MemoryCreated
MemoryUpdated
MemoryDeleted
```

---

# 26. Event 的未来用途

未来可以让：

```text
Audit
Plugin
Cloud
Automation
Index
Search
```

订阅这些事件。

目标架构：

```text
LoomaCore
   │
   ├── Repository
   │
   ├── Audit
   │
   └── Domain Event
          │
          ├── Plugin
          ├── Cloud
          ├── Automation
          └── Index
```

---

# 27. 暂时不要实现复杂 Event Bus

不要在 v0.2 引入：

```text
Kafka
RabbitMQ
Message Queue
复杂 Broker
Distributed Event Bus
```

Looma 是 Local-First Desktop Application。

当前阶段只需要：

```text
Event Contract
+
简单 in-process dispatch
```

如果当前代码结构甚至不需要 dispatch，也可以先定义 event 类型和 emission boundary。

---

# 28. P2 — Plugin Contract

Plugin Runtime 暂时不实现。

但需要考虑未来 Plugin Boundary。

建议未来可能存在：

```text
Importer
Exporter
MetadataProvider
CloudProvider
SearchProvider
AutomationProvider
```

---

# 29. Plugin 原则

当前：

```text
Trait / Interface
    ↓
Internal Implementation
```

未来：

```text
Plugin Runtime
    ↓
Trait / Interface
```

不要现在就引入：

```text
dynamic DLL loading
ABI compatibility
sandbox
marketplace
plugin installation system
```

---

# 30. P2 — Managed Asset

当前如果已经存在：

```text
AssetSource::Managed
```

可以保留。

但 v0.2 不要求完整实现 Managed Asset。

Looma 当前核心仍然应该是：

```text
Reference Mode
```

也就是：

> 文件继续由用户自己的文件系统管理，Looma 管理索引、关系、元数据和引用。

---

# 31. P2 — Cloud Provider

CloudStorage Trait 定义完成后：

暂时不要要求：

```text
Google Drive Provider
OneDrive Provider
S3 Provider
WebDAV Provider
```

全部实现。

Provider 可以作为未来独立 milestone。

---

# 32. P3 — 暂停扩张

以下功能明确不进入本次 Architecture Hardening：

```text
OCR
Vector Database
Semantic Search
LLM Agent
AI Chat
Plugin Marketplace
Mobile Client
Remote Looma
Multi-device Sync
Cloud-native Database
```

不是否定这些功能。

只是：

> **它们应该建立在已经稳定的 Core / MCP / Audit / Reference / Manifest / Extension Boundary 之上。**

---

# 33. Intelligence 当前定位

当前 intelligence 已经存在：

```text
filename similarity
folder clustering
relation suggestion
```

这些可以继续保留。

但是定位为：

```text
Deterministic Heuristics
```

或者：

```text
Smart Helpers
```

而不是：

```text
AI Engine
Agent Framework
LLM Layer
```

---

# 34. Intelligence 原则

v0.2：

```text
Deterministic
Local
Explainable
Optional
```

暂时不引入：

```text
LLM dependency
Vector DB dependency
Remote AI dependency
```

未来 AI 可以通过：

```text
MCP
Plugin
External AI Client
```

访问 Looma。

---

# 35. 最终目标架构

v0.2 完成后，希望形成：

```text
                         External AI
                    ┌───────┼────────┐
                    ↓       ↓        ↓
                   DSH  Antigravity  Other AI
                    │       │        │
                    └───────┼────────┘
                            ↓
                           MCP
                            │
                    Permission Boundary
                            │
                    ┌───────┴────────┐
                    │                 │
                 Tools             Resources
                    │                 │
                    └───────┬─────────┘
                            ↓
                        LoomaCore
                            │
          ┌─────────────────┼──────────────────┐
          ↓                 ↓                  ↓
      Domain Model      Services          Extension
          │                 │                  │
          │                 │          ┌───────┼────────┐
          │                 │          ↓       ↓        ↓
          │                 │       Events  Cloud    Plugin
          │                 │
          └─────────────────┼──────────────────┘
                            ↓
                       Repository
                            ↓
                          SQLite
```

---

# 36. Local-First 不变

整个 v0.2 重构过程中必须保持：

```text
No Cloud Required
No Internet Required
No LLM Required
No External AI Required
```

Looma 在纯本地环境下仍然应该可以：

```text
create
read
update
search
relate
backup
export
```

---

# 37. Backward Compatibility

除非存在明确的 bug 或架构冲突：

> **不要随意破坏现有 v0.1 数据。**

优先：

```text
migration
compatibility
adapter
```

而不是：

```text
drop table
rewrite all data
break CLI
break MCP
```

---

# 38. 实施顺序

建议严格按照：

```text
Step 1
Core Facade

↓
Step 2
Migrate Desktop / CLI / MCP to Core

↓
Step 3
MCP Resources

↓
Step 4
MCP Permission Boundary

↓
Step 5
Audit Log

↓
Step 6
ExternalReference

↓
Step 7
Manifest Hardening

↓
Step 8
CloudStorage Trait

↓
Step 9
Domain Event Contract

↓
Step 10
Plugin Contract Documentation
```

---

# 39. 每一步的验收要求

不要只验证：

```text
cargo build
cargo test
```

必须验证：

## Core

```text
CLI → Core → DB
Desktop → Core → DB
MCP → Core → DB
```

不存在新的业务逻辑绕过 Core。

---

## MCP

验证：

```text
tools/list
tools/call

resources/list
resources/read
```

---

## Permission

至少验证：

```text
READ → allowed

METADATA_WRITE → policy controlled

CONTENT_WRITE → policy controlled

DESTRUCTIVE_WRITE → denied by default
```

---

## Audit

验证：

```text
mutation
    ↓
audit record
```

至少覆盖：

```text
create
update
delete
```

---

## ExternalReference

验证：

```text
create
read
update
delete
relation
export
import
```

---

## Manifest

验证：

```text
export
checksum
schema version
restore / validation
```

---

# 40. Definition of Done

只有满足以下条件，才能认为 Looma v0.2 Architecture Hardening 完成：

### P0

* [ ] LoomaCore / VaultContext 已成为业务操作的主要入口
* [ ] CLI 不直接实现重复业务逻辑
* [ ] Desktop 不直接实现重复业务逻辑
* [ ] MCP 不直接操作 Repository / DB 完成业务 mutation
* [ ] MCP Resources 可用
* [ ] MCP Permission Boundary 存在
* [ ] destructive operation 默认受到限制
* [ ] Audit Log 可记录关键 mutation

### P1

* [ ] ExternalReference 模型存在
* [ ] ExternalReference 可被查询 / 导出
* [ ] Manifest 有 schema_version
* [ ] Manifest 有完整性信息
* [ ] CloudStorage Trait 已定义
* [ ] Domain Event Contract 已定义

### P2

* [ ] Plugin Boundary 有明确设计文档
* [ ] Managed Asset 保持未来兼容
* [ ] Cloud Provider 不强制实现

### P3

* [ ] OCR 不进入本阶段
* [ ] Vector DB 不进入本阶段
* [ ] Semantic Search 不进入本阶段
* [ ] LLM Agent 不进入本阶段
* [ ] Plugin Marketplace 不进入本阶段

---

# 41. 非目标

本阶段明确禁止以“顺手”的方式扩大范围：

```text
❌ 不要为了 MCP 引入完整云认证系统
❌ 不要为了 Event 引入消息队列
❌ 不要为了 Plugin 实现动态 DLL Runtime
❌ 不要为了 Search 引入 Vector DB
❌ 不要为了 Intelligence 接入 LLM
❌ 不要为了 Cloud 实现多个 Provider
❌ 不要为了 Audit 实现完整 Event Sourcing
❌ 不要为了 Managed Asset 重构整个文件系统
```

如果某项功能确实需要这些能力：

> 先留下明确 Extension Boundary，暂不实现。

---

# 42. 对现有实现的要求

在开始修改前，先进行一次：

```text
Current Architecture Inventory
```

明确列出：

```text
Current
├── Core
├── Domain
├── Repository
├── CLI
├── Desktop
├── MCP
├── Intelligence
├── Backup
├── Manifest
└── Tests
```

然后标记：

```text
KEEP
REFACTOR
MOVE
DEPRECATE
NEW
```

不要直接大规模重写。

---

# 43. 最重要的原则

Looma 的核心不是：

> “现在拥有多少功能。”

而是：

> **“未来增加多少功能，都不需要推翻底座。”**

因此本阶段优先级：

```text
Architecture
    >
Boundary
    >
Data Integrity
    >
Extensibility
    >
New Features
```

---

# 44. 最终定位

Looma 不应该被设计成：

```text
Todo App
Note App
Photo Manager
File Manager
AI Assistant
Knowledge Base
```

中的某一个。

它更接近：

```text
Personal Digital Operating System
```

核心职责：

```text
Files
Records
Entities
Relations
Memories
References
Timeline
Collections
```

通过统一 Core 暴露：

```text
GUI
CLI
MCP
```

再通过扩展边界连接：

```text
AI
Plugin
Cloud
Automation
```

最终形成：

```text
                         ┌─────────────┐
                         │ External AI │
                         └──────┬──────┘
                                │
                               MCP
                                │
                     ┌──────────┴──────────┐
                     │      LoomaCore      │
                     └──────────┬──────────┘
                                │
             ┌──────────────────┼──────────────────┐
             ↓                  ↓                  ↓
          Records            Assets            Relations
             │                  │                  │
             └──────────────────┼──────────────────┘
                                ↓
                           Local Storage
                                │
                 ┌──────────────┼──────────────┐
                 ↓              ↓              ↓
              Backup          Event          Audit
                 │              │
                 ↓              ↓
              Cloud          Plugin
```

---

# 45. 给实现 Agent 的最终指令

请基于本文件对当前 Looma 项目进行 **Architecture Hardening**。

不要假设现有代码不存在。

不要从零重写。

不要为了满足文档而制造不必要的抽象。

执行顺序：

```text
1. Inventory 当前实现
2. 对照本计划建立 Gap Matrix
3. 优先完成 P0
4. 完成 P0 后运行完整测试
5. 再完成 P1
6. P2 只建立边界
7. P3 不实现
```

每完成一个阶段，都输出：

```text
Implemented
Changed
Not Implemented
Tests
Remaining Gap
```

特别要求：

> **如果当前代码已经存在某项能力，不要重复实现；先判断它是否已经满足本计划的架构要求。**

最终目标不是让报告看起来“全部 PASS”。

最终目标是：

> **代码结构真正满足这些边界，并且通过测试证明。**

**Looma v0.2 的成功标准不是“功能更多”，而是“底座更稳，未来扩展不需要推倒重来”。**
