# Local-First Architecture

---

## Design Goals

1. **Zero external dependencies** — no Docker, no cloud accounts, no Supabase
2. **One installer** — Tauri produces a native app bundle (macOS .dmg, Windows .exe, Linux .AppImage)
3. **Two binaries, clean separation** — desktop app for the visual workspace, daemon for indexing and MCP
4. **Optional local inference** — Ollama/Gemma used when available, not required
5. **Existing engine preserved** — no rewrite of working TypeScript indexing logic

---

## Two-Binary Model

```
┌─────────────────────────────────────────────────────┐
│  Sensei Desktop  (Tauri + SvelteKit)                │
│                                                     │
│  ┌──────────────────────────────────────────────┐  │
│  │ SvelteKit webview                            │  │
│  │  - Workspace: projects, ideas, maturity      │  │
│  │  - Phase pipeline: req → analysis → design   │  │
│  │  - Card workspace: cards, links, prompts     │  │
│  │  - Analytics: FTR, sessions, token costs     │  │
│  └──────────────────────────────────────────────┘  │
│                                                     │
│  Rust backend (Tauri commands)                      │
│   - File system access (open folder, watch files)  │
│   - Git integration (clone, status, diff)          │
│   - SQLite: read directly (display queries)        │
│   - IPC: send commands to daemon                   │
│   - Lifecycle: start/stop daemon process           │
└─────────────────────────────────────────────────────┘
          │ IPC (local socket or HTTP on 127.0.0.1)
          ▼
┌─────────────────────────────────────────────────────┐
│  Sensei Daemon  (TypeScript/Bun)                   │
│                                                     │
│  ┌──────────────────────────────────────────────┐  │
│  │ MCP server (packages/server)                 │  │
│  │  - get_session_context, search, context_pack │  │
│  │  - Used by Claude Code directly              │  │
│  └──────────────────────────────────────────────┘  │
│                                                     │
│  ┌──────────────────────────────────────────────┐  │
│  │ Indexer (packages/engine)                    │  │
│  │  - Scan, parse, symbol extraction            │  │
│  │  - Graph writes to SQLite                    │  │
│  │  - Ollama adapter (optional)                 │  │
│  └──────────────────────────────────────────────┘  │
│                                                     │
│  ┌──────────────────────────────────────────────┐  │
│  │ Collector (packages/collector)               │  │
│  │  - Session events from Claude Code hooks     │  │
│  │  - FTR calculation                           │  │
│  └──────────────────────────────────────────────┘  │
│                                                     │
│  ┌──────────────────────────────────────────────┐  │
│  │ Command API (REST on 127.0.0.1:51790)        │  │
│  │  - POST /index  POST /reindex                │  │
│  │  - POST /cards  PATCH /cards/:id             │  │
│  │  - POST /prompt (built-in commands)          │  │
│  └──────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
          │ reads / writes
          ▼
┌─────────────────────────────────────────────────────┐
│  SQLite  (~/.sensei/sensei.db)                     │
│   - symbols, call_edges, imports (from engine)     │
│   - projects, ideas, cards, phases, links          │
│   - sessions, turns, ftr_scores                    │
│   - libraries, lib_docs                            │
└─────────────────────────────────────────────────────┘
```

---

## Data Layer: SQLite Replaces Supabase

### Why SQLite

| Supabase | SQLite |
|---|---|
| Requires Docker or cloud project | Embedded — a single file |
| Setup: 10-20 minutes | Setup: zero |
| pgvector for embeddings | SQLite-vec or SQLite FTS5 for search |
| RLS for multi-user | Not needed — single-user tool |
| Realtime subscriptions | File-watch polling (adequate for local use) |
| Remote backup built in | Developer backs up their own file |

### Read vs. Write split

**Desktop reads SQLite directly** for all display queries. This makes the UI snappy
without needing a round-trip to the daemon for every navigation action.

**Daemon owns all writes.** Cards, indexing results, session events, FTR scores —
all mutations go through the daemon's command API. This prevents write conflicts and
keeps business logic in one place.

### Database location

```
~/.sensei/sensei.db          — global: sessions, FTR, library index, settings
<repo>/.sensei/index.db      — per-repo: symbols, call graph, embeddings
```

The split keeps per-repo data portable (the repo folder can be moved or shared)
and the global data in a predictable location for backup.

---

## Graph Model

Every entity is a node. Every relationship is a typed edge.

### Node types

| Type | Description |
|---|---|
| `Project` | A git repository registered in sensei |
| `Idea` | A pre-code concept, not yet a repo |
| `Phase` | A named stage within a project or idea |
| `Card` | An atomic unit of thinking within a phase |
| `Symbol` | A code symbol (function, class, type) |
| `File` | A source file |
| `Library` | An indexed third-party or internal library |
| `Decision` | An architectural decision record |
| `Session` | A Claude Code session |

### Edge types

| From | Edge | To | Meaning |
|---|---|---|---|
| Card | `drives` | Card | This requirement drives this design |
| Card | `implements` | Card | This task implements this requirement |
| Card | `references` | Symbol | This card references this code symbol |
| Card | `references` | File | This card references this file |
| Symbol | `calls` | Symbol | This function calls that function |
| Symbol | `exports` | File | This file exports this symbol |
| File | `depends_on` | Library | This file imports from this library |
| Session | `modified` | Symbol | This session changed this symbol |
| Session | `addressed` | Card | This session worked on this card |
| Decision | `informs` | Card | This ADR shapes this design card |

### Graph queries enable built-in commands

The built-in commands are graph traversals, not LLM calls:

- `gap analysis` → cards in Requirements phase with no `drives` edge to Implementation
- `trace [card]` → follow edge chain from card up to requirements and down to symbols
- `find orphans` → symbols with no `references` edge from any card
- `phase summary` → aggregate maturity of all cards in a phase
- `token estimate` → count symbols reachable from task cards, estimate context size

---

## Local Inference (Optional)

Ollama with Gemma 3 (2B or 4B) is used for:

1. **L1 description generation** — one-sentence summaries of symbols during indexing
2. **Card suggestion** — when analyzing a repo, suggesting cards for the Analysis phase
3. **Gap questions** — generating clarifying questions when a requirement is thin

Ollama is **detected at runtime**. If not running, these features are skipped or
fall back to heuristics. There is no hard dependency.

The developer's primary AI reasoning partner remains Claude (via Claude Code + MCP).
Local inference handles the cheap, repetitive, offline-capable tasks.

---

## Package Map: Existing → New

| Package | Current role | New role |
|---|---|---|
| `packages/shared` | Supabase client, types | SQLite client, types (Supabase references removed) |
| `packages/engine` | Indexing, context packs, ranking | Same — adapts to write SQLite instead of Supabase |
| `packages/server` | MCP server | Same — unchanged, still serves Claude Code |
| `packages/collector` | Event daemon | Same — writes to SQLite instead of Supabase |
| `packages/cli` | sensei CLI | Retained for headless/CI use; `sensei init`, `sensei index` |
| `apps/dashboard` | SvelteKit standalone app | Embedded in Tauri webview; new routes for workspace/cards |
| `apps/desktop` (new) | — | Tauri app: Rust backend + SvelteKit webview |

---

## What the Tauri Rust Layer Owns

Tauri commands (Rust functions callable from the SvelteKit frontend) handle:

- **File system dialogs** — open folder, select repo
- **Git status** — read working tree status, branch, remote URL
- **Process management** — start/stop/restart the daemon, check if Ollama is running
- **Auto-launch** — register daemon as a login item on macOS/Windows
- **System tray** — quick access to workspace, daemon status
- **Auto-update** — Tauri updater checks GitHub Releases

Business logic stays in TypeScript. Rust is strictly the system integration layer.

---

## Telemetry (Deferred)

When telemetry is added (Phase 6+), it will be:

- **Opt-in only** — explicit user consent, clear explanation of what is sent
- **Minimal** — repo visibility (public/private), language detected, session count; no code, no prompts
- **Enterprise signal** — number of repos, frequency of use, git remote domain — used to identify
  potential enterprise users for a future team/enterprise offering
- **Open protocol** — the telemetry payload schema is published and the sending code is visible

Nothing is sent until the user explicitly enables it.
