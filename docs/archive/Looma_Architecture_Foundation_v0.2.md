# Looma — Architecture Foundation
## Local-First Personal Digital Vault
### Architecture Baseline v0.2

> **Project name:** Looma  
> **Project type:** Local-first personal digital asset, entity, memory and timeline system  
> **License:** MIT (proposed)  
> **Primary platform:** Windows first; architecture should remain cross-platform  
> **Core stack:** Rust + Tauri 2 + React/TypeScript + SQLite/FTS5  
> **AI strategy:** No built-in AI. AI connects through standard external interfaces, primarily MCP.

---

# 0. Executive Definition

Looma is a **local-first, user-owned, extensible personal digital vault**.

It organizes a user's digital world without requiring the user to surrender ownership of the underlying files.

The system combines:

```text
Assets
Entities
Relations
Collections
Memories
Timeline
External References
Search
Backup
Open Interfaces
```

The central design idea is:

> **Looma is a data system first and an application second.**

The desktop UI is one client.

The CLI is another client.

MCP is another client.

Future HTTP/API clients may be added.

AI is an external consumer of Looma, not part of Looma Core.

---

# 1. Core Philosophy

## 1.1 Local First

Looma must work completely offline.

The application must not require:

- user accounts
- cloud services
- AI providers
- remote databases
- internet access

for normal local operation.

---

## 1.2 User Owns the Data

Looma must never become the sole owner of the user's information.

The user should be able to:

- inspect files directly
- export structured data
- rebuild indexes
- restore backups
- migrate away from Looma

The application must not create artificial lock-in.

---

## 1.3 Cloud Is Backup, Not Dependency

The preferred architecture is:

```text
                         USER DEVICE
┌──────────────────────────────────────────────────────┐
│                                                      │
│  Looma Core                                           │
│      │                                               │
│      ├── SQLite runtime index                        │
│      ├── JSON / Markdown structured data              │
│      ├── asset references                            │
│      └── local files                                 │
│                                                      │
└─────────────────────────┬────────────────────────────┘
                          │
                    Optional Backup
                          │
                          ▼
┌──────────────────────────────────────────────────────┐
│                    CLOUD                             │
│                                                      │
│  database backup                                     │
│  entities                                            │
│  memories                                            │
│  relations                                           │
│  collections                                         │
│  configuration                                       │
│  manifests                                           │
│  optional thumbnails                                 │
│                                                      │
│  NOT normally uploaded:                              │
│  large images / videos / audio / original media      │
│                                                      │
└──────────────────────────────────────────────────────┘
```

A cloud outage must never make the local vault unusable.

---

# 2. The Most Important Architectural Decision

## Looma Core is independent from the UI and AI.

The architecture must be:

```text
                         Looma Core
                            │
          ┌─────────────────┼─────────────────┐
          │                 │                 │
          ▼                 ▼                 ▼
       Desktop             CLI               MCP
       Tauri              Terminal           AI
          │                 │                 │
          └─────────────────┼─────────────────┘
                            │
                            ▼
                    Same Core Operations
```

There must be one source of business logic.

Do NOT implement:

```text
GUI logic
+
CLI duplicate logic
+
MCP duplicate logic
```

Instead:

```text
GUI ──┐
CLI ──┼──> Looma Core ──> Storage
MCP ──┘
```

This is a foundational requirement.

---

# 3. External Interface Strategy

Looma should expose multiple interfaces.

## 3.1 Core API

The internal stable API used by all clients.

Conceptual operations:

```text
AssetService
EntityService
RelationService
MemoryService
CollectionService
SearchService
TimelineService
BackupService
TaskService
```

The Core API owns validation, permissions, persistence and domain behavior.

---

## 3.2 CLI

The CLI provides direct human/script access.

Initial conceptual commands:

```bash
looma init
looma scan <path>
looma search <query>
looma asset list
looma asset get <id>
looma entity list
looma entity get <id>
looma memory create
looma collection list
looma backup
looma export
looma restore
looma doctor
```

CLI must call Looma Core.

---

## 3.3 MCP Server

MCP is a first-class external interface.

The purpose is:

> Allow external AI agents and MCP-compatible clients to safely inspect and modify Looma.

Potential clients:

```text
DSH
Antigravity
Claude Code
Gemini-based agents
Other MCP-compatible agents
Custom agents
Future automation
```

The MCP server must call Looma Core.

AI must never directly access SQLite.

AI must never directly manipulate Looma's internal files.

---

# 4. MCP Design

## 4.1 Read Tools

Initial conceptual tools:

```text
search_assets
get_asset
list_assets

search_entities
get_entity
list_entities

search_memories
get_memory

list_collections
get_collection

get_timeline

list_relations
get_relations
```

---

## 4.2 Metadata Write Tools

```text
create_entity
update_entity

create_memory
update_memory

create_relation
delete_relation

create_collection
update_collection

add_to_collection
remove_from_collection

update_asset_metadata
```

---

## 4.3 Potentially Dangerous Tools

These must NOT be unrestricted by default:

```text
delete_asset
move_asset
rename_asset
copy_asset
bulk_update
bulk_delete
```

The permission model must distinguish:

```text
read
metadata_write
content_write
destructive_write
```

Recommended defaults:

```text
read              = enabled
metadata_write    = explicitly configurable
content_write     = disabled by default
destructive_write = disabled by default
```

The user must be able to grant narrower permissions to an MCP client.

---

# 5. MCP Resources

MCP Resources should expose stable read-only representations where useful.

Conceptual URI scheme:

```text
looma://asset/<id>
looma://entity/<id>
looma://memory/<id>
looma://collection/<id>
looma://timeline/<period>
```

Resources should expose structured data suitable for AI context.

---

# 6. MCP Security Model

External agents are untrusted clients.

Never assume:

```text
"AI knows what it is doing."
```

Instead:

```text
External Agent
      │
      ▼
MCP Permission Layer
      │
      ▼
Looma Core Validation
      │
      ▼
Storage
```

The Core remains authoritative.

Potential future capabilities:

```text
per-client permissions
read-only mode
approval-required writes
operation audit log
temporary permission grants
path-level restrictions
```

Do not implement a complicated permission UI in v0.1 unless required.

The architecture must leave room for it.

---

# 7. Optional HTTP API — Future

A REST/HTTP API is NOT required for v0.1.

However, the architecture should not prevent:

```text
Looma Core
     │
     ├── Desktop
     ├── CLI
     ├── MCP
     └── Future HTTP API
```

Do not introduce HTTP merely because it might be useful later.

---

# 8. AI Philosophy

## AI is not part of Looma Core.

Do not build:

```text
AI chatbot
AI subscription system
built-in model provider
mandatory cloud inference
```

Instead:

```text
AI Agent
   │
   │ MCP
   ▼
Looma
```

This allows the user to choose:

```text
DSH
Antigravity
Claude
Gemini
DeepSeek
Local models
Future agents
```

without changing Looma.

---

# 9. Core Domain Model

Looma's core data model consists of:

```text
Asset
Entity
Relation
Memory
Collection
ExternalReference
Timeline Event
```

The system should remain domain-neutral.

---

# 10. Asset

An Asset represents a digital resource.

Examples:

```text
image
video
audio
document
archive
3D model
screenshot
PDF
external URL
```

Conceptual structure:

```text
Asset
├── id
├── kind
├── source
├── path / URL
├── size
├── hash
├── created_at
├── modified_at
├── metadata
├── status
└── timestamps
```

Assets may be:

```text
local
managed
external
missing
```

---

# 11. Asset Storage Modes

## 11.1 Reference Mode — Default

The user already owns the file.

Example:

```text
D:\Anime\BLEACH\TYBW\episode.png
```

Looma records:

```text
path
metadata
hash
relationships
```

but does not move or duplicate the file.

---

## 11.2 Managed Mode — Optional

The user explicitly asks Looma to manage/copy the asset.

Example:

```text
Vault/
└── managed-assets/
```

The UI must clearly indicate whether an asset is referenced or managed.

Never silently move user files.

---

# 12. Entity

An Entity represents something meaningful to the user.

Examples:

```text
Anime
Manga
Game
Book
Movie
Person
Device
Software
Project
Website
Organization
Place
Personal project
```

The Core must NOT hard-code these domain types.

Example:

```json
{
  "id": "bleach",
  "type": "anime",
  "title": "BLEACH",
  "properties": {
    "status": "watching",
    "rating": 5
  }
}
```

The `type` and `properties` are extensible.

---

# 13. Dynamic Entity Types

Users should eventually be able to define custom entity types.

For example:

```text
Device
├── CPU
├── GPU
├── RAM
├── purchase_date
└── warranty

Project
├── repository
├── language
├── status
├── started
└── finished

Anime
├── status
├── rating
├── episodes
└── favorite_characters
```

The Core should operate on the generic Entity abstraction.

Domain-specific behavior belongs in schemas/plugins.

---

# 14. Relation

Relations connect objects.

Examples:

```text
BLEACH
 ├── has_asset → screenshot
 ├── contains → Ichigo
 ├── related_to → Tite Kubo
 ├── mentioned_in → memory
 └── part_of → collection
```

Relations must be generic.

A relation should not require moving or duplicating files.

---

# 15. Memory

Memory represents a personal record.

Examples:

```text
journal
note
reflection
development log
watching note
learning note
life snapshot
```

Preferred durable representation:

```text
Markdown + metadata
```

Example:

```text
memories/
└── 2026/
    └── 2026-10-03.md
```

Memories can relate to:

```text
entities
assets
projects
collections
timeline events
```

---

# 16. Collection

Collection is a user-defined grouping.

Examples:

```text
Favorite Anime
2026 Projects
BLEACH Screenshots
Things To Watch
Vibecoding Projects
```

Collections must not require file duplication.

A collection is a logical grouping.

---

# 17. ExternalReference

Represents an object that lives outside Looma.

Examples:

```text
GitHub repository
Bilibili video
website
cloud document
Steam game
article
Google Drive document
```

Example:

```json
{
  "type": "github_repository",
  "title": "Personal Vault",
  "url": "https://github.com/...",
  "external_id": "...",
  "metadata": {}
}
```

External references should not require importing the external content.

---

# 18. Timeline

Timeline provides a chronological view across the vault.

Potential sources:

```text
memory.created
asset.created
entity.created
project.started
project.finished
watching.started
watching.completed
manual snapshot
```

Timeline is a view over data, not necessarily a separate duplicated data store.

---

# 19. Life Snapshot

Life Snapshot is a special but generic use case of Memory + Timeline + Relations.

Example:

```text
2026-10

Work
├── AI Engineer
├── ADK
└── Vibecoding

Interests
├── BLEACH
├── Games
└── Anime

Projects
├── Personal Vault
├── PC Control Center
└── Other projects

Media
├── photos
├── videos
└── music
```

Do not create a separate hard-coded LifeSnapshot subsystem in v0.1.

Implement it using existing primitives.

---

# 20. Filesystem Scanner

Scanning must be incremental.

Track enough information to avoid unnecessary work:

```text
path
size
mtime
file type
filesystem identity where available
hash when necessary
```

Basic logic:

```text
unchanged
    → skip

new
    → index

modified
    → update

missing
    → mark missing

restored
    → reconnect
```

Hashing should be asynchronous.

Do not block the UI.

---

# 21. Background Task Engine

Expensive operations must execute through background tasks.

Task examples:

```text
scan
hash
thumbnail
metadata extraction
OCR
backup
restore
import
export
AI processing
```

Conceptual Task:

```text
Task
├── id
├── type
├── state
├── progress
├── created_at
├── started_at
├── finished_at
├── error
└── cancellation
```

Tasks should support:

```text
progress
cancellation
retry
error reporting
```

---

# 22. Search

Search should cover:

```text
assets
entities
entity properties
relations
memories
collections
external references
timeline
```

Initial implementation:

```text
SQLite
+
FTS5
+
normal indexes
```

Do not introduce Elasticsearch or a vector database.

Future semantic search can be added only if a real use case appears.

---

# 23. Database Strategy

Use SQLite as the runtime database.

SQLite provides:

```text
relational storage
indexes
JSON support
FTS5
transactions
single-file database
```

However:

> SQLite is not the only representation of user data.

Important structured data should be exportable to:

```text
JSON
Markdown
YAML where appropriate
```

The runtime index should be reconstructible where practical.

---

# 24. Database Migrations

Every schema change must have an explicit migration version.

Example:

```text
migration 001
migration 002
migration 003
```

Never silently change the schema.

Migration failures must be explicit and recoverable.

---

# 25. Backup Format

Backup must be application-independent as much as practical.

Example:

```text
backup/
├── manifest.json
├── vault.db
├── config/
├── entities/
├── memories/
├── collections/
└── metadata/
```

Manifest:

```text
vault_version
schema_version
backup_timestamp
application_version
included_datasets
checksums
```

Original large media is excluded by default.

---

# 26. Cloud Adapter

Cloud providers must be abstracted.

Conceptual interface:

```text
CloudStorage
├── upload
├── download
├── list
├── delete
├── exists
└── verify
```

Possible future providers:

```text
Google Drive
OneDrive
WebDAV
S3-compatible storage
Google Cloud Storage
NAS
```

Do not hard-code a provider into the Core.

---

# 27. Sync vs Backup

These are different concepts.

## Backup

```text
Local
  ↓
Cloud snapshot
```

Cloud does not need to be live.

## Sync

```text
Device A
   ↕
Device B
```

requires conflict resolution and is significantly more complex.

Do NOT implement full multi-device sync in v0.1.

Architecture may support it later.

---

# 28. Plugin Architecture

Plugins extend Looma without modifying Core.

Potential categories:

```text
Importer
Exporter
MetadataProvider
AIProvider
CloudProvider
MediaProcessor
EntityTypeProvider
CommandProvider
MCPToolProvider
```

Initial plugin API must remain small.

Do not build a plugin marketplace in v0.1.

---

# 29. Event System

Stable events may include:

```text
asset.created
asset.updated
asset.deleted

entity.created
entity.updated
entity.deleted

relation.created
relation.deleted

memory.created
memory.updated

collection.created
collection.updated

scan.started
scan.completed

backup.started
backup.completed
```

Plugins and adapters may subscribe to stable events.

Do not expose unstable internal implementation details.

---

# 30. Desktop Architecture

Recommended:

```text
Tauri 2
│
├── Rust Core
│
└── React / TypeScript UI
```

React should primarily handle:

```text
views
components
interaction
layout
visual state
```

Rust should handle:

```text
filesystem
database
search
tasks
media
backup
business logic
permissions
```

Avoid putting core business logic into React.

---

# 31. UI Philosophy

The interface should be:

```text
simple
fast
quiet
modern
information-dense when useful
not overloaded
```

Initial navigation:

```text
Home
Assets
Entities
Collections
Timeline
Memories
Settings
```

Global search:

```text
Ctrl/Cmd + K
```

Detail views should follow a common model:

```text
Title
Type
Metadata
Relations
Assets
Timeline
Notes
```

Do not build a dashboard full of meaningless statistics.

---

# 32. CLI / GUI / MCP Consistency

All three must ultimately call the same domain operations.

Example:

```text
GUI:
User clicks "Add to Collection"

CLI:
looma collection add ...

MCP:
add_to_collection(...)
```

All three eventually execute:

```text
CollectionService.add_asset(...)
```

This is a hard architectural rule.

---

# 33. Security Boundary

The architecture must distinguish:

```text
UI
CLI
MCP
Plugin
Core
Filesystem
```

External callers must not bypass Core validation.

Never allow:

```text
MCP → raw SQLite
MCP → arbitrary filesystem mutation
Plugin → arbitrary Core internals
React → direct database
```

unless explicitly designed and permissioned.

---

# 34. Auditability

Future write operations should be auditable.

Potential record:

```text
operation
actor
timestamp
target
before
after
result
```

Especially useful for:

```text
MCP writes
bulk updates
destructive operations
automated plugins
```

A full audit UI is not required in v0.1.

---

# 35. Privacy Baseline

Default:

```text
No telemetry
No analytics
No mandatory account
No automatic cloud upload
No automatic AI upload
No public sharing
```

User must explicitly opt into external data processing.

---

# 36. Technology Stack

## Desktop

Tauri 2

## Backend

Rust

## Frontend

React + TypeScript

## Database

SQLite + FTS5

## Structured data

JSON / Markdown / optional YAML

## Package/build

Cargo + standard TypeScript ecosystem

## MCP

Official MCP protocol/SDK-compatible server implementation

The MCP implementation should remain an adapter over Looma Core.

---

# 37. Proposed Repository

```text
looma/
│
├── apps/
│   └── desktop/
│       ├── src/
│       └── src-tauri/
│
├── crates/
│   ├── core/
│   ├── database/
│   ├── scanner/
│   ├── search/
│   ├── media/
│   ├── metadata/
│   ├── tasks/
│   ├── backup/
│   ├── plugin/
│   ├── cli/
│   └── mcp/
│
├── packages/
│   ├── ui/
│   ├── types/
│   └── api/
│
├── plugins/
│
├── schemas/
│
├── docs/
│
├── scripts/
│
└── tests/
```

Only create modules when a real boundary exists.

Do not create abstractions just because they sound architecturally impressive.

---

# 38. Recommended Initial Rust Boundaries

The initial Rust workspace should have clear responsibility boundaries.

```text
core
    domain concepts + service interfaces

database
    SQLite + migrations + repositories

scanner
    filesystem discovery

metadata
    file/media metadata extraction

media
    thumbnails / previews / media-specific processing

search
    indexing + querying

tasks
    background execution

backup
    export / restore / manifests

plugin
    plugin contracts/runtime boundary

cli
    terminal adapter

mcp
    MCP adapter
```

The exact crate split may be simplified if early implementation shows that some modules are too small.

---

# 39. MVP Scope

## Phase 0 — Foundation

Implement only:

```text
Rust workspace
Tauri application
React UI
SQLite
migration system
logging
configuration
Core API
```

Verify:

```text
build
launch
database creation
basic command invocation
```

---

## Phase 1 — Assets

```text
directory selection
incremental scanning
asset records
metadata
hash
basic thumbnails
asset browser
basic search
```

---

## Phase 2 — Entities

```text
generic entities
dynamic properties
relations
collections
entity detail view
```

---

## Phase 3 — Memories

```text
Markdown memories
timeline
entity ↔ memory relations
life snapshot through existing primitives
```

---

## Phase 4 — Backup

```text
manifest
export
restore
local backup
cloud adapter abstraction
```

---

## Phase 5 — External Interfaces

```text
CLI
MCP server
MCP permissions
stable Core API
events
```

Note:

The interfaces should be architected from the beginning, but they do not all need to be feature-complete in Phase 0.

---

## Phase 6 — Optional Intelligence

Only after the core system is stable:

```text
AI tagging
OCR
semantic search
automatic relation suggestions
AI-assisted organization
```

No built-in AI chat is required.

---

# 40. Explicit Non-Goals

Do not implement merely because technically possible:

```text
social network
cloud-only architecture
built-in AI chatbot
subscription system
recommendation feed
media streaming service
full photo editor
full video editor
password manager
financial manager
calendar replacement
project management suite
full note-taking replacement
plugin marketplace
multi-device realtime sync
```

Every feature must justify itself against the core model.

---

# 41. AI Coding Agent Rules

Any coding agent (Antigravity, DSH, Claude Code, Gemini-based agent, etc.) must follow these rules.

1. Read this architecture document before changing architecture.
2. Inspect the repository before writing code.
3. Prefer small isolated changes.
4. Do not refactor unrelated code.
5. Do not introduce dependencies without justification.
6. Do not bypass Looma Core from UI, CLI, plugins or MCP.
7. Do not directly manipulate SQLite from external interfaces.
8. Do not silently move/copy user media.
9. Do not add cloud requirements to local features.
10. Identify the layer of every new feature.
11. Add tests for core behavior.
12. Preserve migration/data compatibility.
13. Keep public interfaces small.
14. Do not add speculative abstractions.
15. Do not add AI functionality unless explicitly requested.
16. Do not expand MVP scope without discussion.
17. Prefer boring, maintainable solutions over clever ones.
18. When uncertain, document the trade-off before implementing a major architectural decision.

---

# 42. Definition of Done — v0.1

The first usable version should be able to:

```text
1. Launch as a native desktop application.
2. Work completely offline.
3. Select local directories.
4. Incrementally scan files.
5. Display indexed media.
6. Search assets.
7. Create generic entities.
8. Attach assets to entities.
9. Create collections.
10. Create Markdown memories.
11. Display a simple timeline.
12. Export structured data.
13. Create a restorable local backup.
14. Keep the UI responsive during indexing.
15. Expose a minimal CLI/Core interface.
16. Expose a minimal read-only MCP interface.
```

Write operations through MCP may initially be limited until the permission model is validated.

---

# 43. Long-Term Architecture

The intended system is:

```text
                         PERSONAL VAULT
                              LOOMA
                                │
               ┌────────────────┼────────────────┐
               │                │                │
             Assets          Entities         Memories
               │                │                │
               └────────────────┼────────────────┘
                                │
                            Relations
                                │
                             Timeline
                                │
                         Search / Query
                                │
                    ┌───────────┴───────────┐
                    │                       │
                 Human                    AI
                    │                       │
             Desktop / CLI            MCP / Agents
                    │                       │
                    └───────────┬───────────┘
                                │
                         User-owned data
```

The purpose is not to predict every future feature.

The purpose is:

> **Make future requirements cheap to add without compromising the core.**

---

# 44. Naming

Current project name:

# Looma

Concept:

> A personal loom that weaves files, entities, memories and time into one local-first digital archive.

Suggested repository names:

```text
looma
looma-core
looma-desktop
looma-mcp
```

Before public release, perform a final check for:

```text
GitHub repository conflicts
crates.io conflicts
npm conflicts
domain availability
trademark conflicts
existing software/product names
```

Do not treat the current name as legally cleared.

---

# 45. First Instruction to the Coding Agent

Before writing application code:

```text
1. Read this entire document.
2. Inspect the current repository.
3. Confirm the proposed architecture.
4. Identify contradictions or missing decisions.
5. Propose the minimum dependency set.
6. Do NOT immediately implement the whole system.
7. Build the smallest bootable Tauri + React + Rust + SQLite application.
8. Verify that it builds and launches.
9. Establish the Core API boundary.
10. Establish the database migration mechanism.
11. Establish the task/error/logging foundation.
12. Only then implement the Asset subsystem.
13. Add MCP only after the Core API is stable enough to expose safely.
```

The coding agent should stop and request human confirmation before making a major architectural change.

---

# 46. Final Architectural Statement

Looma should remain:

```text
small at the core
strong at the boundaries
local by default
open by design
AI-agnostic
cloud-optional
data-portable
performance-oriented
human-readable
```

The application should not attempt to become everything.

It should provide a durable foundation on which useful capabilities can grow.
