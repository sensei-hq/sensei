# Folder Model — Configuration vs Content

> **SUPERSEDED** — This was the initial design doc. The schema has evolved significantly. See [database-schema.md](../database-schema.md) for the current 5-schema architecture.
>
> This doc is retained for historical context on the config-vs-content separation principle and discovery rules, which remain valid.

## Principles

1. **Configuration vs Content** — User intent (what to watch, what to exclude) is configuration. Discovered filesystem state (folders, repos) is content. They live in separate tables.
2. **Content is ephemeral** — Everything in `folders` can be deleted and re-discovered by scanning. Configuration survives wipes.
3. **Projects are independent** — One project per repo by default, but projects can merge (3 repos → 1 project) or split. Projects are not tied to the folder tree.
4. **Depth limits for plain folders** — Git repos are tracked at any depth. Plain folders are only recorded up to depth 2 from root. Gitignored directories never enter the table.

---

## Table Overview

### Configuration tables

| Table | Purpose |
|-------|---------|
| `folders_to_watch` | Watched root directories + exclusion config |
| `services` | External service registry (MCP servers) |

### Content tables

| Table | Purpose |
|-------|---------|
| `folders` | Discovered filesystem tree (parent, folder, git, subtree) |
| `projects` | Independent grouping entity for 1+ folders |
| `libraries` | Known packages and documentation sources |
| `library_pages` | Documentation pages/sections per library |
| `referenced_libraries` | Junction: which folders use which libraries |
| `extensions` | Skills, commands, agents, hooks, plugins |
| `tags` | Controlled vocabulary for tag arrays |

### Views

| View | Purpose |
|------|---------|
| `repositories` | `folders` where kind in (git, subtree) |
| `parent_folders` | `folders` where kind = parent |
| `libraries_in_project` | Unique library set across all folders in a project |

---

## Configuration: `folders_to_watch`

Replaces `scanned_roots`. Each row is a directory the user has pointed sensei at. Exclusions are stored here as a JSON array — they are user configuration, not discovered content.

```
folders_to_watch
├── id            uuid PK
├── path          text UNIQUE        -- absolute path on disk
├── name          text               -- display name
├── note          text               -- user description (e.g. "monorepo root, 3 packages")
├── status        watch_status       -- scanning → watching | paused
├── excluded      jsonb              -- ["node_modules","dist",".git",...]
├── modified_at   timestamptz
└── created_at    timestamptz
```

**Status lifecycle:**
- `scanning` — initial state when root is first added; scan in progress.
- `watching` — scan complete, file watchers active.
- `paused` — watchers stopped; content remains in `folders` but is not updated.

**Exclusion behaviour:**
- Adding an entry to `excluded` → delete matching folders + all children from `folders`.
- Removing an entry from `excluded` → trigger re-scan of that subtree, add back folders matching discovery criteria.

---

## Configuration: `services`

External service registry. Currently MCP servers only, extensible via `protocol` field.

```
services
├── id              uuid PK
├── name            text UNIQUE          -- e.g. "postgres-mcp"
├── display_name    text                 -- "PostgreSQL MCP"
├── publisher       text
├── protocol        text                 -- mcp (supports http + stdio transports)
├── kind            service_kind         -- data | api | devtool | service
├── summary         text
├── trigger_stacks  text[]               -- which stacks recommend this
├── tools_count     integer
├── verified        boolean
├── installed       boolean
├── config          jsonb                -- {transport, command, args, url, env, ...}
├── icons           jsonb
├── tags            text[]
├── modified_at     timestamptz
└── created_at      timestamptz
```

---

## Content: `folders`

The discovered filesystem tree. Every entry was found by scanning a watched root.

```
folders
├── id            uuid PK
├── root_id       uuid FK → folders_to_watch(id)  -- which root this belongs to
├── parent_id     uuid FK → folders(id)            -- nullable, null = direct child of root
├── project_id    uuid FK → projects(id)           -- nullable
├── kind          folder_kind                      -- parent | folder | git | subtree
├── name          text                             -- display name
├── path          text                             -- relative to root
├── abs_path      text UNIQUE                      -- absolute path on disk
├── remote_urls   jsonb                            -- [{name, url}, ...]
├── icons         jsonb                            -- {emoji, devicon, custom}
├── props         jsonb                            -- extensible metadata (see below)
├── tags          text[]                           -- quick-access tags
├── modified_at   timestamptz
└── created_at    timestamptz
```

**`kind` enum (`folder_kind`):**

| Kind | Meaning |
|------|---------|
| `parent` | Organisational folder at depth 1 from root |
| `folder` | Plain folder at depth 2 from root (max depth for non-git) |
| `git` | Git repository — tracked at any depth |
| `subtree` | Git subtree within a git repo |

**No `root` kind** — roots live in `folders_to_watch` (configuration), not here.
**No `excluded` kind** — exclusions live in `folders_to_watch.excluded` (configuration), not here.

---

## Content: `projects`

Independent grouping entity. Not a view — projects have their own lifecycle.

```
projects
├── id             uuid PK
├── name           text                     -- display name
├── description    text
├── client         text                     -- client or owner name
├── maturity       project_maturity         -- discovery → active → maintenance → archived
├── goal           text                     -- what this project is for
├── icon           jsonb                    -- {kind, value, bg, fg}
├── stack          jsonb                    -- {languages, frameworks, runtimes, services}
├── commands       jsonb                    -- {dev, test, build, lint, ...}
├── links          jsonb                    -- [{id, kind, label, url}]
├── guidelines     jsonb                    -- [{id, rule, source}]
├── preferred_acp  text                     -- claude-code | cursor | zed | ...
├── tags           text[]                   -- quick-access tags
├── modified_at    timestamptz
└── created_at     timestamptz
```

**`maturity` enum (`project_maturity`):**

| Stage | Meaning |
|-------|---------|
| `discovery` | Just scanned, minimal data — initial state |
| `active` | In active development |
| `maintenance` | Stable, low-activity |
| `archived` | No longer worked on |

**Relationship to folders:**
- `folders.project_id` → `projects.id` (many folders → one project)
- On first scan, each git/subtree folder gets its own auto-created project (1:1 default)
- Users can merge projects (reassign `folder.project_id` to a shared project, delete orphaned project)
- Users can split projects (create new project, reassign selected folders)
- The project `name` is initially derived from the parent folder name or git remote

**Stack derivation:**
- Each git/subtree folder carries its own `props.stack` (per-repo detected stack)
- The project `stack` is the union of all member repos' stacks
- Updated by the indexer when any member repo is re-scanned

---

## Content: `libraries`

Known packages and documentation sources. Consolidates the former `libraries`, `lib_meta`, and `shared_libs` tables.

```
libraries
├── id            uuid PK
├── kind          library_kind         -- detected | imported
├── name          text
├── ecosystem     text                 -- npm | pypi | cargo | go | docs
├── version       text
├── description   text
├── source_type   text                 -- llms.txt | http | local
├── base_url      text                 -- root URL for fetching
├── local_path    text                 -- local path if source_type = local
├── homepage_url  text
├── docs_url      text
├── page_count    integer              -- denormalized count of library_pages
├── embedding     vector(384)          -- on description
├── icons         jsonb
├── props         jsonb                -- {llms_txt, llms_txt_fetched_at, ...}
├── tags          text[]
├── indexed_at    timestamptz
├── modified_at   timestamptz
└── created_at    timestamptz
```

- **detected** — discovered from Cargo.toml, package.json, etc.
- **imported** — manually registered (internal SDKs, llms.txt sources)

---

## Content: `library_pages`

Documentation pages/sections for libraries. Consolidates the former `lib_doc_sections`, `shared_lib_sections`, and `lib_docs` tables.

```
library_pages
├── id            uuid PK
├── library_id    uuid FK → libraries(id)
├── title         text
├── url           text               -- remote URL if fetched over HTTP
├── local_path    text               -- filesystem path if sourced locally
├── description   text
├── content       text               -- full page content
├── source_type   text               -- llms.txt | http | local
├── component     text               -- sub-topic within the library
├── embedding     vector(768)        -- for semantic search
├── fetched_at    timestamptz
├── modified_at   timestamptz
└── created_at    timestamptz
```

---

## Content: `referenced_libraries`

Junction table linking folders (git/subtree repos) to their library dependencies. Replaces the former `repo_libraries` and absorbs config from `repo_libs`.

```
referenced_libraries
├── folder_id     uuid FK → folders(id)     ┐
├── library_id    uuid FK → libraries(id)   ┘ composite PK
├── version_used  text                      -- from package.json, Cargo.toml, etc.
├── props         jsonb                     -- {source, usage_count, skill_path, ...}
├── modified_at   timestamptz
└── created_at    timestamptz
```

**View: `libraries_in_project`** — unique set of libraries across all folders in a project. Joins `referenced_libraries` → `folders` → `libraries`, grouped by `project_id`.

---

## Content: `extensions`

Skills, commands, agents, hooks, and plugins. Plugins are containers — other extensions reference them via `plugin_id` (self-ref). Content is the markdown body, frontmatter parsed into `props`.

```
extensions
├── id            uuid PK
├── plugin_id     uuid FK → extensions(id)  -- self-ref: null = standalone
├── kind          extension_kind            -- plugin | skill | command | agent | hook
├── name          text UNIQUE
├── version       text
├── description   text
├── content       text                      -- markdown body (round-trips to .md files)
├── props         jsonb                     -- parsed frontmatter + config
├── scope         text                      -- global | project | folder
├── enabled       boolean
├── source        text                      -- builtin | marketplace | local
├── icons         jsonb
├── tags          text[]
├── modified_at   timestamptz
└── created_at    timestamptz
```

---

## Content: `tags` (controlled vocabulary)

```
tags
├── tag           text PK
├── category      text               -- e.g. "stack", "domain", "status"
├── created_at    timestamptz
```

Tag is unique — category is a soft hint for UI grouping/autocomplete. Tables that need tags carry a `tags text[]` column for quick access.

---

## Per-Repo Attributes (in `folders.props`)

For `kind = 'git'` or `kind = 'subtree'`, the `props` jsonb carries:

```json
{
  "role": "backend",
  "lang": "Rust · Axum",
  "files": 412,
  "loc": "18k",
  "stack": {
    "languages": ["Rust", "TypeScript"],
    "frameworks": ["axum", "sqlx"],
    "runtimes": ["tokio 1.36"]
  },
  "libs": ["tokio", "serde", "axum"],
  "indexed_at": "2026-04-23T10:30:00Z",
  "last_error": null,
  "duplicate_of": null,
  "label": "api-server"
}
```

| Key | Type | Description |
|-----|------|-------------|
| `role` | string | backend, frontend, library, docs, infra, unknown |
| `lang` | string | Primary language display string (e.g. "Rust · Axum") |
| `files` | number | Total file count in repo |
| `loc` | string | Lines of code estimate (e.g. "18k LOC") |
| `stack` | object | `{languages, frameworks, runtimes}` — per-repo stack |
| `libs` | string[] | Detected library/package names |
| `indexed_at` | timestamp | Last successful index time |
| `last_error` | string? | Last indexing error message |
| `duplicate_of` | uuid? | If this is a dup, reference to canonical folder |
| `label` | string? | Optional user-provided label |

---

## Per-Project Attributes

| Field | Source | Description |
|-------|--------|-------------|
| `icon` | User or auto-detected | `{kind:"kanji", value:"工", bg:"var(--shu-soft)", fg:"var(--shu)"}` |
| `stack` | Derived from member repos | Union of all repos' `props.stack` |
| `commands` | Parsed from config files | `{dev:"bun run dev", test:"cargo test"}` |
| `links` | Auto-discovered + manual | `[{id, kind:"docs"\|"dashboard"\|"issues", label, url}]` |
| `guidelines` | Sessions + manual | `[{id, rule:"All handlers wrap ApiError", source:"house-style"}]` |
| `preferred_acp` | User choice | Default AI tool: "claude-code", "cursor", etc. |
| `goal` | README / sensei.json / user | What this project is for |

---

## Discovery Rules

When a watched root is scanned, the scanner walks the directory tree and applies these rules:

### What gets added to `folders`

```
Given: root = /Users/jerry/Developer

Depth 0: root itself → NOT added (lives in folders_to_watch)
Depth 1: /Users/jerry/Developer/clients/      → kind = parent
Depth 1: /Users/jerry/Developer/sensei-dev/    → kind = git (has .git)
Depth 2: /Users/jerry/Developer/clients/acme/  → kind = folder (if not git)
Depth 2: /Users/jerry/Developer/clients/acme/  → kind = git   (if has .git)
Depth 3+: only if git repo or subtree found
```

### What never enters `folders`

- Directories matching `folders_to_watch.excluded` patterns
- Gitignored directories: `node_modules`, `dist`, `build`, `target`, `.git`, `.next`, `.svelte-kit`, `__pycache__`, `.venv`, `venv`
- `.`-prefixed directories (`.claude`, `.sensei`, `.idea`, etc.) — not added by default
- Plain directories at depth 3+ that contain no git repos beneath them

### Git subfolder handling

Subfolders inside a git repo are **not** added to `folders` unless they are a subtree (have their own `.git` or are registered as a git subtree). The repo's internal structure is handled by the indexer (symbols, chunks, etc.), not the folder model.

### Subtree detection

A subtree is identified by:
1. Git subtree config in `.git/config` or `.gitmodules`
2. A nested `.git` directory within an already-tracked git folder

Subtrees get `kind = 'subtree'` and `parent_id` pointing to their containing git folder.

---

## Scanning Lifecycle

```
User adds root path
        │
        ▼
  ┌──────────────────┐
  │ folders_to_watch  │  status = 'scanning' (initial)
  │   + excluded      │  excluded = ['node_modules','dist',...]
  └───────┬──────────┘
          │
          ▼
    Scanner walks tree
          │
          ├── depth 1: add parent/git folders
          ├── depth 2: add folder/git (non-git stops here)
          ├── depth 3+: only git/subtree
          │
          ▼
  ┌─────────────┐
  │   folders    │  kind = parent|folder|git|subtree
  │  + project   │  auto-created 1:1 for git/subtree
  └──────┬──────┘
         │
         ▼
    folders_to_watch.status → 'watching'
         │
         ▼
    For git/subtree folders:
    indexer runs → symbols, chunks, embeddings, etc.
```

### Exclusion flow

```
User adds "experiments" to folders_to_watch.excluded
        │
        ▼
  DELETE FROM folders
   WHERE root_id = :root
     AND (path = 'experiments' OR path LIKE 'experiments/%')
        │
        ▼
  Cascade deletes: symbols, chunks, embeddings, etc.
  (orphaned projects cleaned up separately)

User removes "experiments" from folders_to_watch.excluded
        │
        ▼
  Re-scan subtree at root_path/experiments/
        │
        ▼
  INSERT discovered folders back (same discovery rules)
  Auto-create projects for any new git/subtree folders
```

---

## Migration from current schema

| Old table | New location |
|-----------|-------------|
| `scanned_roots` | `folders_to_watch` |
| `repos` | `folders` (kind = git/subtree) |
| `excluded_paths` | `folders_to_watch.excluded` jsonb |
| `projects` | `projects` (enriched with icon, stack, links, guidelines, maturity, preferred_acp) |
| `tags` (entity_type, entity_id, tag) | `tags` (controlled vocabulary) + `tags text[]` on tables |
| `libraries` | `libraries` (consolidated with lib_meta + shared_libs) |
| `lib_meta` | `libraries` |
| `shared_libs` | `libraries` |
| `lib_doc_sections` | `library_pages` |
| `shared_lib_sections` | `library_pages` |
| `lib_docs` | `library_pages` |
| `repo_libraries` | `referenced_libraries` |
| `repo_libs` | absorbed into `libraries` + `referenced_libraries.props` |

All 20+ tables that FK to `repos(id)` will FK to `folders(id)` instead. The column name stays `repo_id` to minimise churn — it just points at a folder row where kind ∈ {git, subtree}.

---

## Code Graph (Relational, structured for future Cypher)

The code graph uses relational tables with joins and recursive CTEs. This covers 90%+ of queries (1-2 hop: callers, callees, fan-in/fan-out). If deep traversal becomes a bottleneck, Apache AGE can be layered on top of the same tables without migration.

### Graph tables

| Table | Purpose |
|-------|---------|
| `symbols` | Nodes — functions, classes, interfaces, types, consts, enums, rationale |
| `call_edges` | Directed edges — caller → callee with confidence |
| `symbol_map` | Per-file symbol summaries at L0/L1/L2 resolution |
| `chunks` | Content chunks with 384-dim embeddings (HNSW) |
| `embeddings` | Per-file 384-dim embeddings for semantic ranking |
| `detected_patterns` | Patterns detected in code with lifecycle state |

### Graph capabilities

**Visualization (3 lens modes):**
- Call graph — force-layout with nodes sized by fan-in/fan-out
- Matrix — heatmap of file-to-file connections
- Clusters — community bubbles (Leiden algorithm, computed offline)

**Overlays (5 types):**
- Rework heat — files edited in 3+ sessions in 7 days
- Duplicate clusters — structurally similar code with confidence scores
- Patterns — where detected patterns apply
- God-nodes / hotspots — highest fan-in + fan-out symbols
- Stale / drift — files untouched while related files evolved

**Key queries (all 1-2 hop JOINs):**
- `get_callers(symbol)` — who calls this?
- `get_callees(symbol)` — what does this call?
- `get_duplicates(folder)` — structurally similar code
- God nodes — `GROUP BY caller_id + COUNT` on call_edges
- Community membership — stored as `community_id` on symbols (batch-computed)

### Confidence tagging on edges

| Level | Meaning | Source |
|-------|---------|--------|
| `extracted` | Direct import, explicit function call | AST parser |
| `inferred` | Embedding similarity above threshold, BM25 co-occurrence | Indexer |
| `ambiguous` | Flagged during drift detection or gap analysis | Analyzer |

### Pattern lifecycle

Patterns flow through a lifecycle: **suggested → gap → rule**

| State | Meaning |
|-------|---------|
| `suggested` | Emerging pattern, seen in 2+ places, not yet adopted |
| `gap` | Pattern recommended but absent from the codebase |
| `rule` | Promoted to project rule — enforced and checked |

Anti-patterns are tracked separately with severity (high/medium/low) and cross-link to the constructive pattern that would fix them.

### Community detection

Leiden algorithm runs offline as a batch job. Results stored as `community_id` integer on symbols. Communities correspond to modules, subsystems, or cross-cutting concerns. Used for:
- Graph cluster visualization
- Context pack assembly (load whole community)
- Bridge node identification (symbols connecting communities)
