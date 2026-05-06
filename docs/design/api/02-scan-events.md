---
name: "Scan SSE Event Contract"
description: StateEvent format for project assembly and scan activity — daemon emits, app consumes
date: 2026-04-28
type: design
traces:
  - design/api/00-api-surface-overview.md
  - design/02-desktop/ui-state-pattern.md
---

# Scan SSE Event Contract

## Endpoint

`GET /api/scan/events` — single SSE stream, two entity types: `project` and `activity`.

## Event format

```json
{ "action": "add|update|remove|set", "entity": "project|activity", "data": T | T[] }
```

---

## Entity: `project`

Project cards in the left panel. Each project has folders with individual progress.

```ts
interface ScanProject {
  id: string;
  name: string;
  status: 'scanning' | 'indexing' | 'active' | 'failed';
  folders: ScanProjectFolder[];
  autoDetected: boolean;
  confidence: 'high' | 'medium' | 'low';
}

interface ScanProjectFolder {
  id: string;
  name: string;              // e.g. lumen-app
  path: string;              // absolute path
  stack: string[];           // e.g. ['typescript'], ['rust']
  filesTotal: number;
  filesCompleted: number;
  status: 'discovered' | 'queued' | 'indexing' | 'indexed' | 'failed';
}
```

### Derived for project card

```
Lumen Studio                         ← project.name
~/code/lumen · 3 folders · 2 ready   ← DERIVED commonParent(folders) · folders.length · folders.filter(indexed).length
ACTIVE                               ← project.status
1,456 / 1,747                        ← sum(filesCompleted) / sum(filesTotal)

lumen-app      TypeScript   ██████████ ← folder.name, folder.stack, folder progress %
lumen-canvas   Rust         ████████░░
lumen-shell    291f         ░░░░░░░░░░ ← folder showing queued file count
```

Project path is derived from `commonParent(project.folders.map(f => f.path))`. Not stored on the project.

### Project events

```json
// Auto-detected during scan
{ "action": "add", "entity": "project", "data": {
    "id": "p1", "name": "Lumen Studio", "status": "scanning",
    "folders": [
      { "id": "f1", "name": "lumen-app", "path": "~/code/lumen/lumen-app", "stack": ["typescript"], "filesTotal": 0, "filesCompleted": 0, "status": "discovered" },
      { "id": "f2", "name": "lumen-canvas", "path": "~/code/lumen/lumen-canvas", "stack": ["rust"], "filesTotal": 0, "filesCompleted": 0, "status": "discovered" }
    ],
    "autoDetected": true, "confidence": "high"
  }
}

// Folder queued for indexing
{ "action": "update", "entity": "project", "data": {
    "id": "p1", "folders": [
      { "id": "f1", "name": "lumen-app", "path": "...", "stack": ["typescript"], "filesTotal": 842, "filesCompleted": 0, "status": "queued" }
    ]
  }
}

// Folder progress update (batched — daemon sends periodically, not per file)
{ "action": "update", "entity": "project", "data": {
    "id": "p1", "folders": [
      { "id": "f1", "name": "lumen-app", "path": "...", "stack": ["typescript"], "filesTotal": 842, "filesCompleted": 612, "status": "indexing" }
    ]
  }
}

// Folder complete
{ "action": "update", "entity": "project", "data": {
    "id": "p1", "status": "active", "folders": [
      { "id": "f1", "name": "lumen-app", "path": "...", "stack": ["typescript"], "filesTotal": 842, "filesCompleted": 842, "status": "indexed" }
    ]
  }
}

// User mutations
{ "action": "update", "entity": "project", "data": { "id": "p1", "name": "renamed" } }
{ "action": "remove", "entity": "project", "data": { "id": "p2" } }
```

Note: folder updates within a project are partial — only the changed folders are in the `folders` array. The client merges by folder `id`.

---

## Entity: `activity`

Scrolling log in the right panel. Always `action: 'add'` — activity is append-only.

```ts
interface ActivityEvent {
  id: string;                // unique, timestamp-based
  level: 'discover' | 'queue' | 'process' | 'info' | 'success' | 'error';
  message: string;           // human-readable line
  elapsed: number;           // seconds since scan started
  timestamp: number;         // epoch ms
}
```

### Activity events (bottom-up, newest first in UI)

```json
{ "action": "add", "entity": "activity", "data": { "id": "a01", "level": "discover", "message": "~/code/lumen · found root", "elapsed": 0.12, "timestamp": 1714300000120 } }
{ "action": "add", "entity": "activity", "data": { "id": "a02", "level": "discover", "message": "~/code/lumen/lumen-app · found git repo", "elapsed": 0.18, "timestamp": 1714300000180 } }
{ "action": "add", "entity": "activity", "data": { "id": "a03", "level": "discover", "message": "~/code/lumen/lumen-canvas · found git repo", "elapsed": 0.24, "timestamp": 1714300000240 } }
{ "action": "add", "entity": "activity", "data": { "id": "a04", "level": "discover", "message": "~/code/lumen/lumen-shell · found git repo", "elapsed": 0.31, "timestamp": 1714300000310 } }
{ "action": "add", "entity": "activity", "data": { "id": "a05", "level": "queue", "message": "lumen-app · 842 files queued", "elapsed": 0.38, "timestamp": 1714300000380 } }
{ "action": "add", "entity": "activity", "data": { "id": "a06", "level": "queue", "message": "lumen-canvas · 614 files queued", "elapsed": 0.42, "timestamp": 1714300000420 } }
{ "action": "add", "entity": "activity", "data": { "id": "a07", "level": "queue", "message": "lumen-shell · 291 files queued", "elapsed": 0.47, "timestamp": 1714300000470 } }
{ "action": "add", "entity": "activity", "data": { "id": "a08", "level": "process", "message": "lumen-app · 612 / 842 processed", "elapsed": 0.82, "timestamp": 1714300000820 } }
{ "action": "add", "entity": "activity", "data": { "id": "a09", "level": "process", "message": "lumen-canvas · 614 / 614 processed · graph extracted", "elapsed": 1.02, "timestamp": 1714300001020 } }
{ "action": "add", "entity": "activity", "data": { "id": "a10", "level": "process", "message": "lumen-app · 842 / 842 processed · graph extracted", "elapsed": 1.08, "timestamp": 1714300001080 } }
{ "action": "add", "entity": "activity", "data": { "id": "a11", "level": "info", "message": "3 projects detected · 8 folders · 3,214 files indexed", "elapsed": 1.20, "timestamp": 1714300001200 } }
{ "action": "add", "entity": "activity", "data": { "id": "a12", "level": "success", "message": "scan complete · 21s", "elapsed": 1.26, "timestamp": 1714300001260 } }
```

### Activity display

```
SSE · /EVENTS
1.5s                                          ← total elapsed
+1.26s   success   scan complete · 21s
+1.20s   info      3 projects detected · 8 folders · 3,214 files indexed
+1.08s   process   lumen-app · 842 / 842 processed · graph extracted
+1.02s   process   lumen-canvas · 614 / 614 processed · graph extracted
+0.82s   process   lumen-app · 612 / 842 processed
+0.47s   queue     lumen-shell · 291 files queued
+0.42s   queue     lumen-canvas · 614 files queued
+0.38s   queue     lumen-app · 842 files queued
+0.31s   discover  ~/code/lumen/lumen-shell · found git repo
```

Each row: `+elapsed  level  message`

Color per level: discover=sumi-3, queue=amber, process=shu, info=sumi-2, success=jade, error=shu

---

## App state classes

### ScanProjectState

```ts
class ScanProjectState extends ReactiveStageContext<ScanProject> {
  // The project update events contain partial folder arrays.
  // Override update to merge folders by id rather than replace.

  override update(data: ...) {
    // merge folder arrays by folder.id
  }

  // Derived
  // Project path derived from folder paths
  projectPath(p: ScanProject): string { return commonParent(p.folders.map(f => f.path)); }

  // Stats (folders stage provides rootCount separately)
  totalFolders = $derived(this.items.reduce((s, p) => s + p.folders.length, 0))
  readyFolders = $derived(this.items.reduce((s, p) => s + p.folders.filter(f => f.status === 'indexed').length, 0))
  totalFiles = $derived(this.items.reduce((s, p) => s + p.folders.reduce((fs, f) => fs + f.filesTotal, 0), 0))
  completedFiles = $derived(this.items.reduce((s, p) => s + p.folders.reduce((fs, f) => fs + f.filesCompleted, 0), 0))
  scanning = $derived(this.items.some(p => p.status === 'scanning' || p.status === 'indexing'))
  done = $derived(this.items.length > 0 && this.items.every(p => p.status === 'active' || p.status === 'failed'))
}
```

### ScanActivityState

```ts
class ScanActivityState extends ReactiveStageContext<ActivityEvent> {
  // Activity is append-only — only 'add' events
  // Keep last 100 events, display newest first

  recent = $derived(this.items.slice(-100).reverse())
  totalElapsed = $derived(this.items.length > 0 ? this.items[this.items.length - 1].elapsed : 0)
  lastLevel = $derived(this.items.length > 0 ? this.items[this.items.length - 1].level : null)
}
```

### Event routing

```ts
const events = new EventManager<StateEvent<any>>(url, JSON.parse);

events.subscribe(event => {
  if (event.entity === 'project') projectState.apply(event);
  if (event.entity === 'activity') activityState.apply(event);
});
```

## Scan page layout

### Stats bar (top)

```
3 ROOTS  |  10 DISCOVERED  |  3 QUEUED  |  3 PROCESSED
```

| Stat | Source |
|------|--------|
| ROOTS | Watch root count from folders stage (passed as prop, not from SSE) |
| DISCOVERED | activityState: count of events where level = 'discover' and message contains 'found git repo' |
| QUEUED | activityState: count of events where level = 'queue' |
| PROCESSED | activityState: count of events where level = 'process' and message contains 'graph extracted' |

### Left panel: project cards

From `ScanProjectState.items`. Each card shows:
- Project name, derived path, folder count, ready count, status
- File progress: sum(filesCompleted) / sum(filesTotal)
- Folder list with name, stack, individual progress bar

### Right panel: activity feed

From `ScanActivityState.recent` (newest first). Each row:
- `+elapsed` | `level` (color-coded) | `message`

---

## Daemon changes

1. `GET /api/scan/events` — new SSE endpoint
2. Emit `StateEvent` format with `entity: 'project' | 'activity'`
3. Scan pipeline emits `activity` events at each stage (discover, queue, process)
4. After folder grouping, emit `project` add events
5. Folder progress updates batched into `project` update events (not per-file)
6. POST mutations on projects also emit events on the stream
7. Deprecate `/api/tasks/progress`

## REST endpoints

```
POST   /api/projects                              — create project
PUT    /api/projects/{id}                         — update (name, icon)
DELETE /api/projects/{id}                         — delete
POST   /api/projects/{id}/folders                 — add folder
DELETE /api/projects/{id}/folders/{folderId}       — remove folder
POST   /api/projects/merge                        — merge { source, target }
POST   /api/scan                                  — start scan for root paths
```
