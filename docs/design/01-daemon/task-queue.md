---
id: task-queue-architecture
type: design
status: active
created: 2026-04-15
implements:
  - feature: codebase-intelligence
    items: [scanning, indexing, watching, incremental-updates]
---

# Task Queue Architecture

## Problem

The current indexing pipeline is monolithic — one worker processes an entire repo in a single blocking call (`index_repo_with_progress`, 7 passes). This causes:

- **Coarse incremental**: file changes re-process only the changed file but skip hierarchy, CALLS, IMPORTS edge rebuilding
- **Per-repo watchers**: one OS watcher thread per registered project (72 threads for 72 projects)
- **Dirty tracker accumulation**: changes batch for 5 seconds before processing, adding latency
- **No parallelism within a repo**: a 2000-file repo blocks one worker for the entire duration
- **Half-baked deletion handling**: watcher was silently dropping delete events

## Solution

Replace the monolithic indexer with a **hierarchical task queue** where each unit of work is a small, focused task.

## Task Hierarchy

```
scan_root(path)
  └── process_repo(repo_path)              [max N concurrent repos]
        ├── process_folder(folder_path)     [1 per directory]
        │     └── process_file(file_path)   [1 per file — uses language adapter]
        ├── resolve_repo_edges(repo_id)     [barrier — waits for all file tasks]
        └── build_connections(repo_id)      [after resolve — docs↔code, cross-repo]
```

## Task Types

### `scan_root(root_path)`
**Trigger**: User calls `POST /api/scan { root: "/path" }`
**Action**:
- Walk directory tree, find folders with `.git`
- Register each as a project in SQLite
- Persist the root path in store (for watcher recreation on restart)
- For each repo: enqueue `process_repo(repo_path)`

### `process_repo(repo_path)`
**Trigger**: `scan_root` enqueues it, or user calls `POST /api/index`
**Action**:
- Register/update project in SQLite
- Detect git remote URL → check for duplicates
- Detect git subtrees → create auto-solution if found
- Detect workspace members (npm, cargo, pnpm, go.work)
- Create virtual grouping nodes: `repo:`, `code:`, `docs:`
- Create package nodes for workspace members
- Walk directory tree → enqueue `process_folder` per directory
- Enqueue `resolve_repo_edges(repo_id)` as barrier (depends on all folder/file tasks)
- Enqueue `build_connections(repo_id)` (depends on resolve_repo_edges)
**Concurrency**: Max N repos processing simultaneously (e.g. 3)

### `process_folder(folder_path, repo_id, parent_pkg_id)`
**Trigger**: `process_repo` enqueues it
**Action**:
- Create module node (`mod:{repo_id}:{rel_path}`)
- Wire `CONTAINS_MOD` edge from parent package
- Enumerate files in this directory (not recursive — subfolders are separate tasks)
- For each file with a supported extension: enqueue `process_file`
- For each `.md/.mdx` file: enqueue `process_file` (doc/extension classification)

### `process_file(file_path, repo_id, module_id)`
**Trigger**: `process_folder` enqueues it, or file watcher detects change
**Action**:
- Route to language adapter by extension (TypeScript, Python, Rust, Java, etc.)
- Adapter extracts:
  - **Symbols**: functions, methods, classes, interfaces, enums, constants
  - **Raw imports**: `["./bar", "express", "../utils"]` (unresolved path strings)
  - **Raw calls**: `["foo", "bar.baz"]` (unresolved function name strings)
  - **Parent tracking**: which class a method belongs to
- Create hierarchy nodes: `fn:`, `type:`, `file:` with EXPORTS_FN/EXPORTS_TYPE edges
- Store **unresolved references** in a staging table or in-memory map:
  - `(file_id, "imports", "./bar")`
  - `(fn_id, "calls", "helperFn")`
  - `(method_id, "parent", "MyClass")`
- For `.md/.mdx` files: parse frontmatter, classify (doc/extension), extract title
- Create doc/extension node with doc_type, doc_category
- Update manifest entry (mtime + hash)

### `resolve_repo_edges(repo_id)`
**Trigger**: Barrier — runs after ALL file tasks for this repo complete
**Action**:
- Read all unresolved references for this repo
- **IMPORTS**: resolve `"./bar"` → find `file:{abs_path}/bar.ts` → create IMPORTS edge
- **CALLS**: resolve `"helperFn"` → find `fn:...:helperFn:*` → create CALLS edge
- **HAS_METHOD**: resolve `parent "MyClass"` → find `type:...:MyClass:*` → create HAS_METHOD edge
- **CONTAINS_FN**: wire module → function containment edges
- Framework tagging (detect React, Express, etc. from imports)
- Community detection

### `build_connections(repo_id)`
**Trigger**: After `resolve_repo_edges` completes
**Action**:
- **Doc ↔ Code traceability**:
  - SPECIFIES: requirement doc → design doc (name matching)
  - IMPLEMENTS: design doc → code module (content mentions)
  - DOCUMENTS: usage doc → code module (co-location)
  - COVERS: doc → file (backtick path references)
  - MENTIONS_FN: doc → function (backtick identifier references)
- **Cross-repo links** (if repo is in a solution):
  - SHARED_SYMBOLS: functions with same name across repos
  - SHARED_DEPS: repos using same libraries
- **Doc drift**: flag docs whose referenced code has changed

## Task Schema

```rust
pub struct Task {
    pub id: u64,
    pub kind: TaskKind,
    pub repo_id: String,
    pub path: String,              // file/folder/root path
    pub parent_task_id: Option<u64>,
    pub status: TaskStatus,        // Pending, Running, Completed, Failed
    pub depends_on: Vec<u64>,      // won't run until these complete
    pub created_at: Instant,
    pub started_at: Option<Instant>,
    pub completed_at: Option<Instant>,
    pub error: Option<String>,
}

pub enum TaskKind {
    ScanRoot,
    ProcessRepo,
    ProcessFolder,
    ProcessFile,
    DeleteFile,
    DeleteFolder,
    ResolveEdges,
    BuildConnections,
}

pub enum TaskStatus {
    Pending,
    Blocked,       // has unmet dependencies
    Running,
    Completed,
    Failed,
}
```

## Barrier Mechanism

When `process_repo` enqueues folder/file tasks, it also enqueues `resolve_repo_edges` with `depends_on` set to ALL the file task IDs. The queue manager tracks dependency completion:

```
When a task completes:
  1. Mark task as Completed
  2. For each task that depends_on this task:
     - Remove this task from its depends_on list
     - If depends_on is now empty → move from Blocked to Pending
```

Since file tasks are created by folder tasks (not directly by process_repo), the dependency wiring is:
- `process_repo` creates a `resolve_edges` task with status Blocked
- Each `process_file` task, when created, is added to `resolve_edges.depends_on`
- When the last file task completes, `resolve_edges` becomes Pending and gets picked up

## File Watcher Integration

### One watcher per scanned root
- User scans `~/Developer` → watcher started on `~/Developer`
- User scans `~/Work` → second watcher started on `~/Work`
- Scanned roots persisted in SQLite → watchers recreated on daemon restart

### On file change event
```
file_modified(/Users/me/Developer/myrepo/src/foo.ts)
  → identify repo: find registered project whose path is a prefix
  → enqueue process_file(foo.ts, myrepo, module_id)
  → enqueue resolve_repo_edges(myrepo) [depends on this file task]
```

### On file deletion event
```
file_deleted(/Users/me/Developer/myrepo/src/old.ts)
  → identify repo
  → enqueue delete_file(old.ts, myrepo)
    → delete_file action: remove nodes with file=old.ts, cascade-delete edges
  → enqueue resolve_repo_edges(myrepo) [to clean up stale CALLS/IMPORTS]
```

### On folder deletion
```
folder_deleted(/Users/me/Developer/myrepo/src/deprecated/)
  → identify repo
  → enqueue delete_folder(deprecated/, myrepo)
    → delete all nodes with file starting with this path
    → delete module node
  → enqueue resolve_repo_edges(myrepo)
```

## SSE Progress Events

```typescript
interface TaskProgress {
  repo_id: string;
  solution_id?: string;   // if repo is in a solution
  total: number;          // total tasks for this repo
  running: number;        // currently executing
  pending: number;        // waiting to run
  completed: number;      // finished successfully
  failed: number;         // finished with error
  current_file?: string;  // file being processed right now
}
```

Events broadcast on task state transitions:
- `task_queued` → increment total + pending
- `task_started` → decrement pending, increment running
- `task_completed` → decrement running, increment completed
- `task_failed` → decrement running, increment failed

## Concurrency Model

- **Workers**: N worker threads (default 3) pull tasks from queue
- **Repo limit**: max M repos processing concurrently (default 3)
- **File parallelism**: file tasks for the same repo can run in parallel across workers
- **Barrier sequencing**: resolve_edges only runs after all files complete
- **Watcher threads**: 1 per scanned root (not per repo)

## Duplicate Detection

At `process_repo` level:
```
git remote get-url origin → remote_url
store.find_projects_by_remote_url(remote_url)
  → if found and different path: mark new project as duplicate_of existing
  → skip indexing for duplicate
```

## Incremental Update Flow

**Single file modified:**
```
Watcher → process_file(foo.ts) → resolve_repo_edges(myrepo)
```
2 tasks. Sub-second for a single file.

**New folder added:**
```
Watcher → process_folder(new_dir/) → N × process_file → resolve_repo_edges
```
N+2 tasks. Parallel file processing.

**Full re-index:**
```
process_repo → M × process_folder → N × process_file → resolve_edges → build_connections
```
All tasks. But file tasks run in parallel, so wall-clock time is much less than sequential.

## Migration Path

1. Build TaskQueue alongside existing IndexQueue
2. Implement task kinds one at a time (process_file first)
3. Wire watcher to use task queue
4. Remove old: IndexQueue, DirtyTracker, per-repo watchers, monolithic pipeline passes
5. Remove backward-compat wrapper methods from GraphDb

## Files to Create/Modify

| File | Change |
|------|--------|
| `src/tasks/mod.rs` | NEW: Task, TaskKind, TaskStatus, TaskQueue |
| `src/tasks/executor.rs` | NEW: Worker loop, barrier resolution, concurrency limits |
| `src/tasks/handlers.rs` | NEW: Handler functions for each TaskKind |
| `src/tasks/progress.rs` | NEW: SSE progress tracking per repo |
| `src/watcher/root_watcher.rs` | NEW: One watcher per scanned root |
| `src/db/store.rs` | Add: scanned_roots table, persist root paths |
| `src/api/routes.rs` | Update: scan, index, progress endpoints |
| `src/api/server.rs` | Update: spawn root watchers, task workers |
| `src/indexer/pipeline.rs` | Decompose into task handlers |
| `src/indexer/worker.rs` | Replace with tasks/executor.rs |
| `src/indexer/queue.rs` | Replace with tasks/mod.rs |
| `src/watcher/dirty_tracker.rs` | REMOVE |
| `src/watcher/repo_watcher.rs` | REMOVE (replaced by root_watcher) |
