---
name: "Scan Event Flow — Task-Level Spec"
description: Per-task spec — input, behaviour, emitted events conforming to StateEvent types
date: 2026-04-28
type: design
---

# Scan Event Flow — Task-Level Spec

## Prerequisite: Task struct cleanup

Remove `repo_id` from `Task`. It's redundant — folder name is `basename(path)`, DB lookup is by path.

### Before
```rust
pub struct Task {
    pub id: u64,
    pub kind: TaskKind,
    pub repo_id: String,        // ← remove
    pub path: String,
    pub parent_task_id: Option<u64>,
    pub module_id: Option<String>,
    pub branch: Option<String>,
    pub url: Option<String>,
    ...
}
```

### After
```rust
pub struct Task {
    pub id: u64,
    pub kind: TaskKind,
    pub path: String,           // absolute path — name derived via basename()
    pub parent_task_id: Option<u64>,
    pub module_id: Option<String>,
    pub branch: Option<String>,
    pub url: Option<String>,
    ...
}
```

Every handler that needs the folder name calls `Path::new(&task.path).file_name()`. Every handler that needs the DB record calls `pg.get_folder_by_path(&task.path)`.

All `Task::new(kind, repo_id, path)` calls become `Task::new(kind, path)`.

---

## Types

### StateEvent (from types.ts)

```ts
StateEvent<T extends { id: string }> {
  action: 'add' | 'update' | 'remove' | 'set';
  entity: string;           // 'project' | 'activity'
  data: T | T[];
}
```

### ScanProject (entity: "project")

```ts
ScanProject {
  id: string;
  name: string;
  status: ProjectStatus;    // 'scanning' | 'indexing' | 'active' | 'failed'
  autoDetected: boolean;
  confidence: 'high' | 'medium' | 'low';
}
```

No `folders` array. Folders are a separate entity joined by `projectId`.

### ScanFolder (entity: "folder")

```ts
FolderKind = 'git' | 'workspace-member' | 'sibling' | 'standalone';
FolderStatus = 'discovered' | 'queued' | 'indexing' | 'indexed' | 'failed' | 'deferred';

ScanFolder {
  id: string;
  projectId: string;        // FK to ScanProject.id — always a real project ID
  name: string;
  path: string;
  kind: FolderKind;
  stack: string[];
  filesTotal: number;
  filesCompleted: number;
  status: FolderStatus;
}
```

`kind` values:
- `git` — has its own `.git`, will be indexed
- `workspace-member` — subdirectory of a monorepo, part of the parent git folder's index
- `sibling` — non-git folder sharing parent with git folders. `status: 'deferred'`. User can opt in to index.
- `standalone` — non-git folder with no git siblings. `status: 'deferred'`. Flagged for cleanup.

`status: 'deferred'`:
- Not indexed. No file count. `filesTotal: 0, filesCompleted: 0`.
- UI shows muted/greyed, no progress bar, "deferred" badge.
- User action: "Index" button promotes to `status: 'queued'` and triggers ProcessGitFolder.

`projectId` is always a real project ID:
- Siblings get the same project ID as their git folder neighbours.
- Standalone folders get their own single-folder project.

### Three SSE entity types

| Entity | Type | Description |
|--------|------|-------------|
| `project` | `StateEvent<ScanProject>` | Project lifecycle (add, status updates) |
| `folder` | `StateEvent<ScanFolder>` | Folder lifecycle + progress updates |
| `activity` | `StateEvent<ActivityEvent>` | Human-readable log feed |

### Client-side join

```ts
const projects = new ReactiveStageContext<ScanProject>();
const folders = new ReactiveStageContext<ScanFolder>();
const activities = new ReactiveStageContext<ActivityEvent>();

// Event routing
events.subscribe(event => {
  if (event.entity === 'project') projects.apply(event);
  if (event.entity === 'folder') folders.apply(event);
  if (event.entity === 'activity') activities.apply(event);
});

// Derived per project
const projectFolders = (pid: string) => folders.items.filter(f => f.projectId === pid);
const projectPath = (pid: string) => commonParent(projectFolders(pid).map(f => f.path));
const folderCount = (pid: string) => projectFolders(pid).length;
const readyCount = (pid: string) => projectFolders(pid).filter(f => f.status === 'indexed').length;
const totalFiles = (pid: string) => projectFolders(pid).reduce((s, f) => s + f.filesTotal, 0);
const completedFiles = (pid: string) => projectFolders(pid).reduce((s, f) => s + f.filesCompleted, 0);
```

### ActivityEvent

```ts
ActivityEvent {
  id: string;
  level: ActivityLevel;     // 'discover' | 'queue' | 'process' | 'info' | 'success' | 'error'
  message: string;
  elapsed: number;          // seconds since scan started
  timestamp: number;        // epoch ms
}
```

### DB addition

`folders` table needs a `kind` column:

```sql
ALTER TABLE folders ADD COLUMN kind text NOT NULL DEFAULT 'git'
  CHECK (kind IN ('git', 'workspace-member', 'sibling', 'standalone'));
```

---

## Task 1: ScanRoot

### Input
- `path`: root directory to scan (e.g. `~/Code`)

### Algorithm

```
1. Glob all .git directories under root (up to depth N)
2. Parent of each .git = git folder → Set G
3. For each git folder in G:
   - Is it a monorepo? (has workspace config)
     - Cargo.toml with [workspace]
     - package.json with "workspaces"
     - pnpm-workspace.yaml exists
     - go.work exists
   - If monorepo: identify workspace members as sub-folders
4. Compute ancestors of G up to root → Set A (intermediate directories, ignored)
5. Compute parents of G → Set P (project parents for multi-repo grouping)
6. Glob all directories under root
7. Remove G, A, and all subdirectories of G
8. Remove ignored patterns (node_modules, .git, target, dist, build, hidden dirs)
9. Remaining directories → classify:
   - Parent is in P → kind: "sibling", tag with same project as git siblings
   - Else → kind: "standalone", no project
10. Group git folders by parent into projects:
    - Monorepo → project = the git folder, folders = [git folder] + workspace members
    - Multi-repo siblings → project = parent name, folders = all git folders under parent + sibling folders
    - Solo git folder → project = folder name, folders = [git folder]
11. Register watch root in DB
12. Register each git folder in DB (kind: 'git')
13. Register workspace members in DB (kind: 'workspace-member')
14. Register sibling folders in DB (kind: 'sibling')
15. Register standalone folders in DB (kind: 'standalone')
16. Enqueue ProcessGitFolder per git folder
```

### Events emitted

**Per git folder discovered:**
```json
{ "action": "add", "entity": "activity", "data": {
    "id": "<unique>", "level": "discover",
    "message": "~/Code/lumen/lumen-app · git folder",
    "elapsed": 0.18, "timestamp": ...
}}
```

**Per non-git folder flagged:**
```json
{ "action": "add", "entity": "activity", "data": {
    "id": "<unique>", "level": "discover",
    "message": "~/Code/lumen/meeting-notes · non-git sibling",
    "elapsed": 0.20, "timestamp": ...
}}
```

**Per project detected (project entity — no folders array):**
```json
{ "action": "add", "entity": "project", "data": {
    "id": "p-lumen", "name": "lumen", "status": "scanning",
    "autoDetected": true, "confidence": "high"
}}
```

**Per folder in that project (folder entity — separate events):**
```json
{ "action": "add", "entity": "folder", "data": {
    "id": "f-lumen-app", "projectId": "p-lumen", "name": "lumen-app", "path": "...", "kind": "git",
    "stack": [], "filesTotal": 0, "filesCompleted": 0, "status": "discovered"
}}
{ "action": "add", "entity": "folder", "data": {
    "id": "f-lumen-canvas", "projectId": "p-lumen", "name": "lumen-canvas", "path": "...", "kind": "git",
    "stack": [], "filesTotal": 0, "filesCompleted": 0, "status": "discovered"
}}
{ "action": "add", "entity": "folder", "data": {
    "id": "f-meeting-notes", "projectId": "p-lumen", "name": "meeting-notes", "path": "...", "kind": "sibling",
    "stack": [], "filesTotal": 0, "filesCompleted": 0, "status": "deferred"
}}
```

**Monorepo project + folders:**
```json
{ "action": "add", "entity": "project", "data": {
    "id": "p-sensei", "name": "sensei", "status": "scanning",
    "autoDetected": true, "confidence": "high"
}}
{ "action": "add", "entity": "folder", "data": {
    "id": "f-sensei", "projectId": "p-sensei", "name": "sensei", "path": "/code/sensei", "kind": "git",
    "stack": [], "filesTotal": 0, "filesCompleted": 0, "status": "discovered"
}}
{ "action": "add", "entity": "folder", "data": {
    "id": "f-daemon", "projectId": "p-sensei", "name": "daemon", "path": "/code/sensei/daemon", "kind": "workspace-member",
    "stack": [], "filesTotal": 0, "filesCompleted": 0, "status": "discovered"
}}
{ "action": "add", "entity": "folder", "data": {
    "id": "f-app", "projectId": "p-sensei", "name": "app", "path": "/code/sensei/app", "kind": "workspace-member",
    "stack": [], "filesTotal": 0, "filesCompleted": 0, "status": "discovered"
}}
```

**Standalone flagged folder:**
```json
{ "action": "add", "entity": "project", "data": {
    "id": "p-random-notes", "name": "random-notes", "status": "scanning",
    "autoDetected": true, "confidence": "low"
}}
{ "action": "add", "entity": "folder", "data": {
    "id": "f-random-notes", "projectId": "p-random-notes", "name": "random-notes", "path": "...", "kind": "standalone",
    "stack": [], "filesTotal": 0, "filesCompleted": 0, "status": "deferred"
}}
```

**Summary:**
```json
{ "action": "add", "entity": "activity", "data": {
    "id": "<unique>", "level": "info",
    "message": "8 git folders · 2 siblings · 1 standalone · 3 projects detected",
    "elapsed": 0.15, "timestamp": ...
}}
```

### Not emitted by ScanRoot
- No queue events
- No file counts
- No stack detection (that's ProcessGitFolder's job)
- No progress

---

## Task 2: ProcessGitFolder

### Input
- `path`: absolute path to the git folder (name = `basename(path)`)
- From context: project_id this folder belongs to (set by ScanRoot, stored in DB or executor memory)

### Behaviour
1. Detect stack (Cargo.toml, package.json, go.mod, etc.)
2. Detect monorepo workspace config → identify workspace members
3. Walk all indexable files (respecting .gitignore, excluding binaries)
4. Count total files
5. Enqueue ProcessFolder per directory
6. Enqueue ProcessFile per file
7. Enqueue barrier chain: ResolveEdges → ResolveLibs → BuildConnections
8. Detect metadata (icons, external links, summary)

### Events emitted

**Folder update — set stack, filesTotal, status:**
```json
{ "action": "update", "entity": "folder", "data": {
    "id": "f-lumen-app", "projectId": "p-lumen", "name": "lumen-app", "path": "...", "kind": "git",
    "stack": ["typescript"], "filesTotal": 842, "filesCompleted": 0, "status": "queued"
}}
```

**Activity — folder queued:**
```json
{ "action": "add", "entity": "activity", "data": {
    "id": "<unique>", "level": "queue",
    "message": "lumen-app · 842 files queued · typescript",
    "elapsed": 0.38, "timestamp": ...
}}
```

---

## Task 3: ProcessFile

### Input
- `path`: absolute file path
- `parent_task_id`: ProcessGitFolder's task ID (used by executor for progress tracking)
- `module_id`: directory module this file belongs to

### Behaviour
1. Read file, parse with language adapter, extract symbols, write to DB

### Events emitted

**None.** ProcessFile does not emit events. Progress tracking is handled by the executor.

---

## Progress tracking (Executor)

The executor owns progress emission. After completing any ProcessFile task:

1. Look up `parent_task_id` → find the ProcessGitFolder task
2. Increment completed count for that parent (atomic counter per parent)
3. Check batch threshold: emit if 10 files completed since last emit, or 100ms elapsed
4. First completion: folder status `queued` → `indexing`

**Batched folder update** (every 10 files or 100ms):
```json
{ "action": "update", "entity": "folder", "data": {
    "id": "f-lumen-app", "projectId": "p-lumen", "name": "lumen-app", "path": "...", "kind": "git",
    "stack": ["typescript"], "filesTotal": 842, "filesCompleted": 400, "status": "indexing"
}}
```

**Periodic activity** (every ~50 files):
```json
{ "action": "add", "entity": "activity", "data": {
    "id": "<unique>", "level": "process",
    "message": "lumen-app · 400 / 842 processed",
    "elapsed": 0.82, "timestamp": ...
}}
```

### What the executor needs to track per ProcessGitFolder parent

```rust
struct FolderProgress {
    project_id: String,
    folder_id: String,
    folder_name: String,
    folder_path: String,
    kind: FolderKind,
    stack: Vec<String>,
    files_total: u32,
    files_completed: AtomicU32,
    last_emit: AtomicU64,       // timestamp of last SSE emit
    status: FolderStatus,
}
```

Stored in a `HashMap<u64, FolderProgress>` keyed by ProcessGitFolder task ID. Created when ProcessGitFolder runs, consulted on every ProcessFile completion.

---

## Task 4: ResolveEdges

### Input
- `path`: git folder path (same as ProcessGitFolder's path)
- Blocked by: all ProcessFile tasks for this folder

### Behaviour
1. Match unresolved imports to symbols, create edges

### Events emitted

**Folder indexed:**
```json
{ "action": "update", "entity": "folder", "data": {
    "id": "f-lumen-app", "projectId": "p-lumen", "name": "lumen-app", "path": "...", "kind": "git",
    "stack": ["typescript"], "filesTotal": 842, "filesCompleted": 842, "status": "indexed"
}}
```

**Activity — graph extracted:**
```json
{ "action": "add", "entity": "activity", "data": {
    "id": "<unique>", "level": "process",
    "message": "lumen-app · 842 / 842 processed · graph extracted",
    "elapsed": 1.08, "timestamp": ...
}}
```

---

## Task 5: BuildConnections

### Input
- `path`: git folder path
- Blocked by: ResolveLibs

### Behaviour
1. Build cross-folder edges

### Events emitted

**When all git folders in a project have status=indexed (daemon checks after each BuildConnections):**

```json
{ "action": "update", "entity": "project", "data": {
    "id": "p-lumen", "name": "lumen", "status": "active",
    "autoDetected": true, "confidence": "high"
}}
```

```json
{ "action": "add", "entity": "activity", "data": {
    "id": "<unique>", "level": "success",
    "message": "lumen · indexing complete",
    "elapsed": 1.20, "timestamp": ...
}}
```

---

## Task 6: Scan complete

**When all projects are done:**
```json
{ "action": "add", "entity": "activity", "data": {
    "id": "<unique>", "level": "success",
    "message": "scan complete · 21s · 3 projects · 8 folders · 1,747 files",
    "elapsed": 21.0, "timestamp": ...
}}
```

---

## Task 7: RegisterWatchRoots (after scan complete)

### Input
- All root paths from the original `POST /api/scan` request

### Behaviour
1. Upsert each root path into `watch_roots` table (ScanRoot already does this, confirm no duplicates)
2. Signal the file watcher to reload its watch list
3. Watcher starts monitoring all registered roots for file changes

### Events emitted

```json
{ "action": "add", "entity": "activity", "data": {
    "id": "<unique>", "level": "info",
    "message": "3 roots added to watcher",
    "elapsed": 21.1, "timestamp": ...
}}
```

---

## Watcher integration

The file watcher monitors registered roots and enqueues the **same task types** as the scan pipeline. Handlers are stateless — they don't know or care whether they were triggered by a scan or by the watcher.

### Watcher → Task mapping

| File system event | Task enqueued | Notes |
|-------------------|---------------|-------|
| File created | `ProcessFile` | New file in a watched git folder |
| File modified | `ProcessFile` | Re-index changed file (upsert nodes) |
| File deleted | `DeleteFile` | Remove nodes for this file |
| Folder created | `ProcessFolder` | New directory in a watched git folder |
| Folder deleted | `DeleteFolder` | Remove all nodes under this folder |
| New `.git` appears | `ProcessGitFolder` | A new git repo appeared under a watched root |
| `.git` removed | *(TBD)* | Git repo deleted — remove project? flag for review? |

### Shared requirements

1. **Same handlers** — `ProcessFile`, `ProcessGitFolder`, `DeleteFile`, `DeleteFolder` work identically for scan and watcher triggers
2. **Same event_tx** — watcher-triggered tasks emit SSE events through the same broadcast channel
3. **Same DB writes** — all node/edge operations go through PgStore
4. **No scan context assumption** — handlers must NOT reference "scan elapsed time" or "scan start". Activity events from watcher use wall-clock timestamp only, elapsed is 0 or omitted.

### Watcher-emitted events

When the watcher triggers a task, it emits activity events:

```json
{ "action": "add", "entity": "activity", "data": {
    "id": "<unique>", "level": "process",
    "message": "lumen-app · file changed: src/main.ts",
    "elapsed": 0, "timestamp": ...
}}
```

Folder updates follow the same pattern — the executor emits folder status updates as files are re-processed:

```json
{ "action": "update", "entity": "folder", "data": {
    "id": "f-lumen-app", "projectId": "p-lumen", "name": "lumen-app", "path": "...", "kind": "git",
    "stack": ["typescript"], "filesTotal": 843, "filesCompleted": 843, "status": "indexed"
}}
```

Note: `filesTotal` may change (file added/deleted). Since counts are computed from DB, the next query reflects the change. During active re-indexing, the executor tracks ephemeral progress in memory as it does for scan.

### Design constraint

The `elapsed` field on `ActivityEvent` is relative to scan start time. For watcher events, there's no scan — set `elapsed: 0`. The client uses `timestamp` (epoch ms) for watcher events rather than elapsed.

---

## DB schema additions

### folders table

```sql
ALTER TABLE folders ADD COLUMN kind text NOT NULL DEFAULT 'git'
  CHECK (kind IN ('git', 'workspace-member', 'sibling', 'standalone'));
ALTER TABLE folders ADD COLUMN status text NOT NULL DEFAULT 'discovered'
  CHECK (status IN ('discovered', 'queued', 'indexing', 'indexed', 'failed', 'deferred'));
ALTER TABLE folders ADD COLUMN stack jsonb NOT NULL DEFAULT '[]';
ALTER TABLE folders ADD COLUMN project_id uuid REFERENCES projects(id);
```

### No count columns

`files_total` and `files_completed` are NOT stored in the DB:
- During scan: tracked in executor memory (`FolderProgress`)
- After scan: computed from `SELECT COUNT(*) FROM nodes WHERE folder_id = ? AND kind = 'file'`
- This avoids stale counts when watcher adds/deletes files

---

## Folder status transitions

```
discovered → queued      (ProcessGitFolder sets filesTotal)
queued → indexing         (first ProcessFile runs)
indexing → indexed        (ResolveEdges completes)
any → failed             (error in any task)
```

Note: `sibling` and `standalone` folders go directly to `deferred` — they are not indexed unless the user opts in.

## Project status transitions

```
scanning → indexing      (first folder transitions to queued/indexing)
indexing → active        (all git folders reach indexed)
any → failed             (any git folder fails)
```

---

## Client-side state and derivations

### Three stores

```ts
const projects = new ReactiveStageContext<ScanProject>();   // entity: "project"
const folders = new ReactiveStageContext<ScanFolder>();      // entity: "folder"
const activities = new ReactiveStageContext<ActivityEvent>(); // entity: "activity"
```

### Derived per project

```ts
const projectFolders = (pid: string) => folders.items.filter(f => f.projectId === pid);
const gitFolders = (pid: string) => projectFolders(pid).filter(f => f.kind === 'git' || f.kind === 'workspace-member');
const deferredFolders = (pid: string) => projectFolders(pid).filter(f => f.status === 'deferred');
```

### UI element mapping

| UI element | Source |
|------------|--------|
| Project name | `project.name` |
| Project path | `commonParent(projectFolders(pid).map(f => f.path))` |
| Folder count | `projectFolders(pid).length` |
| Ready count | `gitFolders(pid).filter(f => f.status === 'indexed').length` |
| Project status | `project.status` |
| File progress | `sum(gitFolders(pid).filesCompleted) / sum(gitFolders(pid).filesTotal)` |
| Folder progress bar | `folder.filesCompleted / folder.filesTotal` (only for kind=git) |
| Folder stack label | `folder.stack.join(', ')` |
| Folder file label | When queued and no stack: `"{filesTotal}f"` |
| Deferred indicator | `folder.status === 'deferred'` → muted style, "deferred" badge |

### Stats bar

| Stat | Source |
|------|--------|
| ROOTS | Passed from folders stage (not from SSE) |
| DISCOVERED | `activities.items.filter(e => e.level === 'discover').length` |
| QUEUED | `activities.items.filter(e => e.level === 'queue').length` |
| PROCESSED | `folders.items.filter(f => f.status === 'indexed').length` |

---

## Test plan

### Fixture
```
/tmp/test-root/
  proj_a/
    fldr_1/  (.git, Cargo.toml, src/main.rs, src/lib.rs)     → git, rust
    fldr_2/  (.git, package.json, index.ts, utils.ts)          → git, svelte
    fldr_3/  (.git, go.mod, main.go)                           → git, go
    meeting-notes/                                              → sibling (no .git)
  monorepo/  (.git, Cargo.toml with [workspace])
    crates/daemon/
    crates/cli/                                                 → workspace members
  standalone/  (.git, README.md)                                → solo git
  random-docs/                                                  → standalone (no .git, no git siblings)
```

### Test 1: ScanRoot
- Assert: 5 git discover events (fldr_1, fldr_2, fldr_3, monorepo, standalone)
- Assert: 2 non-git discover events (meeting-notes as sibling, random-docs as standalone)
- Assert: project "proj_a" has 4 folders (3 git + 1 sibling)
- Assert: project "monorepo" has 3 folders (1 git + 2 workspace-member)
- Assert: project "standalone" has 1 folder (kind: git, confidence: low)
- Assert: project "random-docs" has 1 folder (kind: standalone, confidence: low)
- Assert: git folders have filesTotal=0, stack=[], status=discovered
- Assert: sibling/standalone folders have status=deferred
- Assert: info activity with summary counts

### Test 2: ProcessGitFolder
- Run ProcessGitFolder for fldr_1
- Assert: queue activity with "fldr_1 · 2 files queued · rust"
- Assert: folder update event with filesTotal=2, stack=["rust"], status=queued
- Assert: 2 ProcessFile tasks enqueued with parent_task_id = ProcessGitFolder's ID

### Test 3: Executor progress emission
- Run ProcessGitFolder + all ProcessFile tasks for fldr_1
- Assert: executor emits folder update with filesCompleted incrementing
- Assert: first completion transitions folder status queued → indexing
- Assert: after all files, filesCompleted = filesTotal

### Test 4: ResolveEdges completion
- Run through ResolveEdges for fldr_1
- Assert: folder update with status → indexed
- Assert: activity "fldr_1 · 2 / 2 processed · graph extracted"

### Test 5: Full pipeline end-to-end
- Run all tasks for all folders to completion
- Assert: all git folders status=indexed
- Assert: sibling/standalone folders still status=deferred
- Assert: each project status=active when all its git folders indexed
- Assert: final activity/success with totals
- Assert: watch roots registered
- Log all events to ~/.sensei/logs/scan-e2e-events.jsonl

### Test 6: Watcher re-index
- After scan, add a new file to fldr_1
- Assert: watcher enqueues ProcessFile
- Assert: folder update event emitted with updated count
- Assert: activity event "lumen-app · file changed: new_file.rs"

### Test 7: Watcher new git folder
- After scan, create a new git folder under a watched root
- Assert: watcher enqueues ProcessGitFolder
- Assert: folder add event with kind=git, status=discovered
- Assert: project updated or new project created
