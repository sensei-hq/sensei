---
id: metadata-model
type: design
implements:
  - feature: codebase-intelligence
    items: [symbol-extraction, incremental-indexing]
---

# Metadata Model

> Defines what data sensei captures, how it is structured, and where it lives. All persistent state is stored in Supabase PostgreSQL. On-disk files under `.sensei/` are generated outputs, not the source of truth.

---

## 1. Overview

Sensei captures four categories of metadata about a codebase: orientation artifacts (project name, stack, entry points, shortcuts), a symbol graph (exported symbols with compressed representations at multiple resolution levels), structural relationships (call edges and import links between files), and indexing fingerprints (content hashes for incremental re-index). Together these enable agents to orient quickly, navigate to relevant code at the right level of detail, and detect when docs have drifted from implementation. All of this data is stored in Supabase and queried at runtime; the `.sensei/` directory holds generated text files derived from that data.

---

## 2. Supabase Schema

All tables live in the `sensei` schema. The schema is applied via migration during `sensei init`.

```sql
-- repos: one row per indexed repository
create table sensei.repos (
  id uuid primary key default gen_random_uuid(),
  path text not null unique,          -- absolute path on disk
  name text not null,
  description text,
  stack text[],
  indexed_at timestamptz,
  created_at timestamptz default now()
);

-- symbols: exported symbols extracted by language adapters
create table sensei.symbols (
  id uuid primary key default gen_random_uuid(),
  repo_id uuid not null references sensei.repos(id) on delete cascade,
  file_path text not null,            -- relative to repo root
  name text not null,
  kind text not null,                 -- 'function' | 'class' | 'interface' | 'type' | 'enum' | 'const'
  signature text,                     -- L0: export declaration line
  io_pattern text,                    -- L1: assignment notation
  logic_flow text,                    -- L2: plain-english steps (Phase 2: LLM-generated)
  unique(repo_id, file_path, name, kind)
);

-- call_edges: function call relationships
create table sensei.call_edges (
  id uuid primary key default gen_random_uuid(),
  repo_id uuid not null references sensei.repos(id) on delete cascade,
  caller_id uuid references sensei.symbols(id) on delete cascade,
  callee_name text not null,
  callee_file text
);

-- imports: import relationships between files
create table sensei.imports (
  id uuid primary key default gen_random_uuid(),
  repo_id uuid not null references sensei.repos(id) on delete cascade,
  source_file text not null,
  target_path text not null,          -- resolved path or package name
  is_external boolean default false,
  unique(repo_id, source_file, target_path)
);

-- scan_state: fingerprints for incremental indexing
create table sensei.scan_state (
  id uuid primary key default gen_random_uuid(),
  repo_id uuid not null references sensei.repos(id) on delete cascade,
  file_path text not null,
  content_hash text not null,
  mtime bigint,
  indexed_at timestamptz default now(),
  unique(repo_id, file_path)
);

-- events: telemetry and session events
create table sensei.events (
  id uuid primary key default gen_random_uuid(),
  repo_id uuid references sensei.repos(id) on delete cascade,
  event_type text not null,
  payload jsonb,
  created_at timestamptz default now()
);
```

### Table responsibilities

| Table | What it holds |
|---|---|
| `repos` | One row per indexed repo: name, description, detected stack, last index timestamp |
| `symbols` | Every exported symbol extracted from source files, with L0/L1/L2 representations |
| `call_edges` | Directed edges from a caller symbol to a callee name (resolved where possible) |
| `imports` | File-level import graph; `is_external` distinguishes npm packages from local files |
| `scan_state` | Per-file content hash and mtime; compared on each scan to skip unchanged files |
| `events` | Telemetry events (index started/completed, errors, agent session markers) |

---

## 3. Orientation Artifacts (Generated Files)

Sensei generates a set of on-disk text files from Supabase data after each successful index run. These files are optimised for agent consumption — they are small, human-readable, and committed to the repo so agents can orient without a live Supabase connection.

| Artifact | Generated from | Purpose |
|---|---|---|
| `.sensei/llmspec.yaml` | `repos` + `symbols` + `scan_state` | Structured orientation for agents |
| `.sensei/llms.txt` | `llmspec.yaml` + README | [llmstxt.org](https://llmstxt.org) standard format |
| `.sensei/stack.md` | `repos.stack` + package analysis | Tech stack summary |
| `.sensei/shortcuts.md` | `package.json` scripts analysis | Dev commands cheat-sheet |
| `.sensei/patterns.md` | Human-edited (auto-scaffolded on first index) | Codebase conventions |

**Supabase is the source of truth.** The generated files are its derived output. On re-index, all generated files except `patterns.md` are overwritten. `patterns.md` is scaffolded once and then owned by humans; it is never overwritten.

`.sensei/config.yaml` is the only on-disk artifact that is _not_ generated — it is the per-project configuration file (Supabase connection, ignore patterns, index options) and is written by `sensei init`.

---

## 4. LLMSpec Format

`.sensei/llmspec.yaml` is the primary orientation artifact. It is the structured equivalent of an OpenAPI spec — compact, queryable by section, and designed to fit within a few hundred tokens.

### Full schema

```yaml
# .sensei/llmspec.yaml — generated by sensei index
# Format version: 1.0

project: string           # Repo/package name (auto)
version: string           # Current version from package.json (auto)
description: string       # One sentence: what this project does (TODO: human review)

stack: string[]           # [typescript, react, postgres, ...] (auto)

entry_points:
  - path: string          # Relative path from repo root (auto)
    role: string          # e.g. "server entry", "route definitions" (TODO: human review)

concepts:                 # Domain terms an LLM must know (human-written)
  - name: string
    definition: string    # One sentence

patterns:                 # Conventions an LLM must follow when editing (human-written)
  - name: string
    files: string         # Path or glob indicating where this pattern applies
    convention: string    # What to do (imperative)

api_surface:              # Key public functions worth knowing (human-curated)
  - name: string
    path: string          # File path
    io: string            # L1 notation: result = fn(input)
    flow: string          # L2 notation: step1 → step2 → output

doc_layers:
  design: string          # Path to design docs / ADRs (auto)
  code: string            # Source root (auto)
  public: string[]        # [README.md, docs/guides/, ...] (auto)

shortcuts:                # Key commands (auto from package.json scripts)
  dev: string
  test: string
  build: string
  index: string
  [key]: string           # Any additional commands
```

### Auto-populated on first index

| Field | Source |
|---|---|
| `project` | `package.json` name, or repo directory name |
| `version` | `package.json` version |
| `stack` | Detected from `repos.stack` in Supabase |
| `entry_points[].path` | Files named `index`, `main`, `app`, `server`, `router` |
| `shortcuts` | `package.json` scripts, Makefile targets |
| `doc_layers.code` | `src/` if present, else repo root |
| `doc_layers.public` | `README.md` if present |

### Requires human review

| Field | Why |
|---|---|
| `description` | One-sentence summary requires judgment |
| `entry_points[].role` | Human description of each entry point |
| `concepts` | Domain terms are not in code |
| `patterns` | Conventions require interpretation |
| `api_surface` | Which functions are "key" requires judgment |
| `doc_layers.design` | Design doc location varies |

Auto-generated values for fields requiring review are set to `"TODO: ..."` on first index.

### Preservation on re-index

On re-index, the generator reads the existing `.sensei/llmspec.yaml`, updates only the auto-populated fields (project, version, stack, shortcuts, doc_layers), and leaves all human-reviewed fields untouched. Fields that were `"TODO: ..."` and have been filled in by a human are never overwritten.

---

## 5. Symbol Resolution Levels

The full definition of L0–L3 levels is in `06-compression.md`. This section records only where each level is stored.

| Level | Name | Stored in |
|---|---|---|
| L0 | Signature | `sensei.symbols.signature` |
| L1 | IO Pattern | `sensei.symbols.io_pattern` |
| L2 | Logic Flow | `sensei.symbols.logic_flow` |
| L3 | Full Source | Not stored — read live from disk |

L2 is populated in Phase 2 via LLM summarisation. In Phase 1, `logic_flow` is null and agents fall back to L3 (reading the file directly) when they need function body detail.

---

## 6. Package Discovery

### PackageAdapter interface

Package adapters run during the Scan stage to discover package boundaries and infer repo-level metadata. The discovered data populates `sensei.repos.stack`, `repos.description`, and the auto-generated sections of `llmspec.yaml`.

```typescript
interface PackageAdapter {
  name: string;                    // "npm", "python", "go", "rust"
  globs: string[];                 // file names this adapter recognises
  detect(files: string[]): boolean;
  extract(dir: string, files: string[], repoPath: string): Promise<PackageInfo>;
}

interface PackageInfo {
  path: string;                    // dir relative to repoPath
  adapter: string;
  name: string;
  description?: string;
  version?: string;
  role?: PackageRole;              // inferred (see below)
  entryPoints: string[];           // main/exports/bin
  stack: string[];                 // inferred from deps
  scripts: Record<string, string>;
  dependencies: Record<string, string>;
  devDependencies: Record<string, string>;
  readme?: ReadmeSummary;
  links: DocLink[];
}

type PackageRole = "cli" | "lib" | "app" | "service" | "ui" | "config" | "docs" | "workspace-root" | "unknown";
```

Repo-level fields (`stack`, `description`) are aggregated from all discovered `PackageInfo` entries and written to `sensei.repos`. Per-package detail (entryPoints, scripts, role) informs the generated orientation files but is not stored as a separate table in Phase 1.

### JS/TS adapter — role inference

```
has "workspaces"                → "workspace-root"  (checked first)
has "bin"                       → "cli"
has "main"/"exports", no "bin"  → "lib"
scripts has "dev" or "start"    → "app"
name contains "ui" or "web"     → "ui"
```

### JS/TS adapter — stack inference

Framework and tool names are matched against known lists in `dependencies` and `devDependencies`. Output is a flat deduped array: e.g. `["typescript", "bun", "vitest", "react", "drizzle-orm"]`. For monorepos, stacks from all child packages are unioned and stored on the root `repos` row.

### Monorepo awareness

A workspace root (`has "workspaces"` in `package.json`) is treated as an orchestrator, not a deployable unit. The adapter discovers child packages by expanding the workspaces glob patterns and processes each as an independent `PackageInfo`. All children are associated with the same `repos` row (there is one `repos` row per repository root, not per package).

### Future adapters

All adapters emit the same `PackageInfo` shape. Planned adapters: Python (`pyproject.toml`, `setup.py`), Go (`go.mod`), Rust (`Cargo.toml`), Ruby (`Gemfile`).

---

## 7. Relationship to Pipeline

This document covers the **data model** — what is captured and where it is stored. `20-pipeline-adapter.md` is the authoritative design for the **processing pipeline** — how the Scan → Parse → Index → Rank → Slice → Assemble stages work, what interfaces each stage exposes, and how language adapters plug in at the Parse stage. Consult that document for the `LanguageParser` interface, `Scanner` interface, and stage-by-stage contracts.
