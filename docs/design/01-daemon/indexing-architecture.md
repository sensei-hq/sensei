---
id: indexing-architecture
type: design
implements:
  - feature: indexing
    items: [project-registration, file-scanning, graph-building, parallel-queue, incremental-watcher, graph-retrieval]
supersedes:
  - 05-indexing.md
  - 12-incremental-indexing.md
---

# Indexing Architecture

> Current as of April 2026. Covers the full pipeline from project registration through graph retrieval.
> Implements a Kuzu embedded graph DB, SQLite-backed parallel indexing queue, and an L0–L5 depth
> retrieval model served over MCP.

---

## 1. Overview

Sensei indexes source code into an embedded property graph (Kuzu) stored at
`~/.sensei/projects/<repoId>/graph.kuzu`. The graph captures functions, types, files, comments,
and the relationships between them (calls, imports, exports, annotations). Agents query the graph
via MCP tools using a tiered depth model — fetching only as much detail as their current task needs.

**Key design principles:**

- **Local-first** — everything lives in `~/.sensei/`, no cloud dependency.
- **Parallel across repos, sequential within one** — the Kuzu connection is not re-entrant; a single
  repo is indexed one file at a time. Multiple repos index concurrently via a WorkerPool.
- **Crash-safe** — a SQLite job queue persists state across daemon restarts. Jobs left in `running`
  state at startup are automatically re-queued.
- **Incremental by default** — a SHA-256 + mtime manifest (`manifest.json`) prevents re-indexing
  unchanged files. `fs.watch` delivers live updates with 300 ms debounce.

---

## 2. Storage Layout

```
~/.sensei/
├── projects.json                    # Registry: [{repoId, name, path, indexedAt}]
├── queue.db                         # SQLite — parallel index job queue (WAL mode)
├── serve.pid                        # PID of the running senseid daemon
└── projects/
    └── <repoId>/
        ├── graph.kuzu/              # Kuzu embedded graph DB
        ├── manifest.json            # mtime + SHA-256 per file (watcher state)
        └── index-state.json         # {lastCommit, indexedAt, repoPath}
```

Each repo gets its own isolated Kuzu database. There is no shared graph between repos.

---

## 3. Graph Schema

```
Nodes
─────
Function  id(PK), name, file, line, sig, body, docstring, complexity, project
Type      id(PK), name, file, line, kind, project
File      id(PK), path, module, lang, project
Comment   id(PK), text, tag, line, file, project

Relationships
─────────────
Function  -[CALLS]->          Function    (weight: DOUBLE)
File      -[IMPORTS]->         File
File      -[EXPORTS_FN]->      Function
File      -[EXPORTS_TYPE]->    Type
Function  -[USES_TYPE]->       Type
Comment   -[ANNOTATES_FN]->    Function
Comment   -[ANNOTATES_TYPE]->  Type
```

Node IDs are deterministic:
- `fn:<absPath>:<name>:<lineStart>`
- `type:<absPath>:<name>:<lineStart>`
- `file:<absPath>`

This allows idempotent `MERGE` — re-indexing a file updates nodes in place without creating duplicates.

---

## 4. Full Indexing Flow

### 4.1 Project Registration

A project becomes known to sensei when it is imported via the desktop app or registered via CLI. The
desktop app `POST /api/projects` immediately, which writes an entry to `~/.sensei/projects.json`. At
that point `indexedAt` is `undefined` — marking it as unindexed.

```mermaid
sequenceDiagram
    actor User
    participant Desktop
    participant Server as senseid (HTTP :7744)
    participant PF as projects.json

    User->>Desktop: Import repo
    Desktop->>Server: POST /api/projects {repoId, path, name}
    Server->>PF: append entry (indexedAt: undefined)
    Server-->>Desktop: {ok: true}
```

### 4.2 Daemon Startup & Reconciliation

On `sensei serve` / `senseid start`, the daemon performs a **desired-state reconciliation** pass:

1. Reads `projects.json`
2. Enqueues any project without `indexedAt` into the SQLite job queue
3. Launches a `watchRepo` watcher for **every** project (indexed or not)

```mermaid
flowchart TD
    A[senseid starts] --> B[Read projects.json]
    B --> C{Any unindexed?}
    C -- yes --> D[Enqueue each into IndexQueue]
    C -- no --> E[Skip]
    D --> E
    E --> F[Start watchRepo for ALL projects]
    F --> G[WorkerPool.start — poll every 2s]
    G --> H[Server accepting connections on :7744]
```

### 4.3 Parallel Queue & WorkerPool

`IndexQueue` (`packages/server/src/index-queue.ts`) is a SQLite table with WAL mode:

```
index_jobs(id, repo_id UNIQUE, repo_path, status, attempts, created_at, updated_at, error)
```

`WorkerPool` runs up to `min(4, cpuCount)` concurrent workers. Each worker:

1. Calls `queue.next()` — atomically claims the oldest `pending` job and marks it `running`
2. Calls `indexRepo(repoId, repoPath)` — full graph build for that repo
3. On success: `queue.markDone()` + updates `indexedAt` in `projects.json`
4. On failure: `queue.markFailed()` — if `attempts < 3`, status reverts to `pending` for retry;
   otherwise permanently `failed`

Crash recovery: at startup, any job stuck in `running` is reset to `pending`.

```mermaid
flowchart LR
    subgraph SQLite queue.db
        J1[job: repoA pending]
        J2[job: repoB pending]
        J3[job: repoC pending]
    end

    subgraph WorkerPool max=min4,cpus
        W1[Worker 1]
        W2[Worker 2]
        W3[Worker 3]
        W4[Worker 4]
    end

    J1 --> W1
    J2 --> W2
    J3 --> W3

    W1 -->|done| PJ[projects.json\nindexedAt = now]
    W2 -->|failed attempt 1| J2R[re-queue pending]
    W3 -->|done| PJ
```

On-demand re-indexing is also available: `POST /api/index {repoId, repoPath, force}` enqueues
immediately, respecting the deduplication logic (`force=true` resets a completed job).

---

## 5. Per-Repo Indexing (`indexRepo`)

`indexRepo` (`packages/graph-indexer/src/indexer.ts`) performs three sequential passes over a
single repo. Sequential-within-repo is intentional — Kuzu connections are not thread-safe.

```mermaid
flowchart TD
    A[indexRepo called] --> B[getOrCreateDb — open graph.kuzu]
    B --> C[ensureSchema — CREATE NODE/REL TABLES IF NOT EXISTS]
    C --> D[fast-glob: collect .ts/.tsx files\nexclude node_modules, dist, spec, d.ts]

    D --> E[Pass 1: File indexing loop]
    E --> F{For each file}
    F --> G[TypeScriptAdapter.parse — AST extract symbols, imports, edges]
    G --> H[mergeNode File]
    H --> I{For each symbol}
    I --> J[function/method/\ncomponent/hook]
    I --> K[type/interface/\nenum/class]
    J --> L[mergeNode Function\nwith sig, body, docstring, complexity]
    L --> M[mergeEdge EXPORTS_FN]
    K --> N[mergeNode Type]
    N --> O[mergeEdge EXPORTS_TYPE]
    M --> P[collect callEdges for pass 3]
    O --> P
    P --> F
    F -->|all files done| Q[Pass 2: IMPORTS edges]

    Q --> R{For each file result}
    R --> S[resolve relative import path\ntry .ts / .tsx / index.ts]
    S --> T[mergeEdge IMPORTS File→File]
    T --> R
    R -->|done| U[Pass 3: CALLS edges]

    U --> V{For each callEdge}
    V --> W[look up callee id in fnIdByName map]
    W --> X[mergeEdge CALLS Function→Function weight=1.0]
    X --> V
    V -->|done| Y[write index-state.json\nlastCommit + indexedAt]
    Y --> Z[close conn + db\nreturn IndexResult]
```

**Why three passes?**

- Pass 1 must complete before passes 2 and 3 because IMPORTS and CALLS edges reference nodes that
  may not exist yet (defined in a different file).
- The `fnIdByName` map is built from all pass-1 results before any CALLS edges are written.

---

## 6. Incremental Indexing (Watcher)

`watchRepo` (`packages/graph-indexer/src/watcher.ts`) handles ongoing changes after the initial
full index. It uses a manifest (SHA-256 + mtime per file) to track what has changed.

### 6.1 Initial Rescan

On first `watchRepo` call, a full `doRescan` runs before the `fs.watch` listener starts. This
catches anything that changed while the daemon was offline:

- Files in manifest but not on disk → `deleteFileFromGraph` + remove from manifest
- Files on disk with changed mtime/hash → delete old nodes, re-index file

### 6.2 Live Change Handling

```mermaid
sequenceDiagram
    participant FS as fs.watch (recursive)
    participant WR as watchRepo
    participant MF as manifest.json
    participant KZ as Kuzu graph

    FS->>WR: file change event (absPath)
    WR->>WR: filter: must match include/exclude globs
    alt file deleted
        WR->>KZ: deleteFileFromGraph (DETACH DELETE)
        WR->>MF: remove entry + save
    else file modified/created
        WR->>WR: add to pendingPaths set
        WR->>WR: reset 300ms debounce timer
        note over WR: waits for more events
        WR->>WR: debounce fires
        WR->>WR: stat + SHA-256 hash each path
        WR->>WR: compare hash to manifest
        alt hash unchanged
            WR->>MF: update mtime only
        else hash changed
            WR->>KZ: deleteFileFromGraph (clear old nodes)
            WR->>KZ: indexSingleFile (insert new nodes)
            WR->>MF: update mtime + hash
        end
        WR->>WR: invoke onUpdate callback
    end
```

The debounce window (default 300 ms) coalesces rapid bursts (e.g. bulk saves, `git checkout`)
into a single re-index pass.

---

## 7. Graph Retrieval — L0–L5 Depth Model

Agents retrieve symbols via MCP tools that query Kuzu with progressive depth. Requesting deeper
levels costs more tokens but reveals more context.

| Level | Data returned | Typical use |
|-------|---------------|-------------|
| L0 | name, kind, file, line | Symbol existence check, navigation |
| L1 | + sig, docstring | Understanding a symbol's interface |
| L2 | + callers, callees, usedTypes | Impact analysis, tracing call chains |
| L3 | + imports, importedBy (file graph) | Module boundary analysis |
| L4 | (reserved — semantic search) | — |
| L5 | + body, annotated comments | Full implementation detail, WHY tags |

### 7.1 MCP Tools

| Tool | What it does |
|------|--------------|
| `get_symbol` | Fetch one symbol at requested depth (layers.ts) |
| `search_code` | Name-contains search over Function + Type nodes |
| `get_bearings` | Returns symbol count, file count, community map (top-level dir clusters) |
| `get_complexity` | Lists functions sorted by cyclomatic complexity |
| `index_repo` | On-demand trigger: POST /api/index then poll for completion |

### 7.2 Community & God-Node Analysis (REST)

`GET /api/graph?repoId=<id>&repoPath=<path>` returns a summary used by the desktop app:

- **Communities** — functions grouped by top-level directory (2-level max). Each community gets a
  color and a list of its top god-nodes.
- **God nodes** — top 20 functions by combined in+out degree across all CALLS edges.
- **Rationale** — all `Comment` nodes tagged `WHY`, `DECISION`, `HACK`, or `NOTE`.

```mermaid
flowchart LR
    MCP[MCP Client\nClaude Agent] -->|get_symbol depth=2| MS[mcp-server.ts]
    MS -->|MATCH Function CALLS| KZ[(Kuzu graph)]
    KZ --> MS
    MS --> MCP

    DA[Desktop App] -->|GET /api/graph| SV[serve.ts]
    SV -->|MATCH Function degree| KZ
    KZ --> SV
    SV --> DA
```

---

## 8. End-to-End Flow Summary

```mermaid
sequenceDiagram
    actor Dev
    participant Desktop
    participant Server as senseid :7744
    participant Queue as IndexQueue\n(queue.db)
    participant Pool as WorkerPool\nmin(4,cpus)
    participant IR as indexRepo\n(graph-indexer)
    participant KZ as Kuzu\ngraph.kuzu
    participant Watch as watchRepo\n(watcher)
    participant MCP as MCP Server\n(mcp-server.ts)
    participant Agent as Claude Agent

    Dev->>Desktop: Import repo /path/to/myapp
    Desktop->>Server: POST /api/projects {repoId, path}
    Server->>Server: write projects.json (no indexedAt)

    Note over Server,Pool: On startup (or next reconciliation)
    Server->>Queue: enqueue(repoId, path)
    Server->>Watch: watchRepo(repoId, path) — start watcher

    Pool->>Queue: next() — claim job
    Queue-->>Pool: job {repoId, repoPath}
    Pool->>IR: indexRepo({repoId, repoPath})
    IR->>KZ: ensureSchema
    IR->>IR: fast-glob scan *.ts/*.tsx
    loop Pass 1 — per file
        IR->>IR: TypeScriptAdapter.parse (AST)
        IR->>KZ: mergeNode File, Function, Type
        IR->>KZ: mergeEdge EXPORTS_FN/TYPE
    end
    IR->>KZ: Pass 2 — mergeEdge IMPORTS
    IR->>KZ: Pass 3 — mergeEdge CALLS
    IR-->>Pool: IndexResult {filesIndexed, ...}
    Pool->>Server: mark done
    Server->>Server: write indexedAt to projects.json

    Note over Watch,KZ: After initial index — live updates
    Dev->>Dev: edit src/foo.ts
    Watch->>Watch: fs.watch event → debounce 300ms
    Watch->>KZ: deleteFileFromGraph(foo.ts)
    Watch->>KZ: indexSingleFile(foo.ts)
    Watch->>Watch: update manifest.json

    Note over MCP,Agent: Query time
    Agent->>MCP: get_symbol("myFunction", depth=2)
    MCP->>KZ: MATCH Function CALLS callers/callees
    KZ-->>MCP: rows
    MCP-->>Agent: SymbolResult {sig, callers, callees}
```

---

## 9. Non-Functional Properties

| Property | Implementation |
|----------|----------------|
| Parallelism | WorkerPool: up to min(4, cpuCount) repos indexed concurrently |
| Crash recovery | SQLite `running` → `pending` reset on daemon restart |
| Retry | Up to 3 attempts per job before permanently `failed` |
| Idempotency | All writes use Kuzu `MERGE` — safe to re-run |
| Change detection | SHA-256 hash + mtime; hash is authoritative (handles git checkout) |
| Deduplication | `UNIQUE` on `repo_id` in queue; `force=true` to override |
| Token efficiency | L0–L5 depth — agents request only what they need |
| Isolation | Each repo has its own Kuzu DB; no cross-contamination |
