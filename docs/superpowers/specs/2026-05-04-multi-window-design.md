# Multi-Window Design — Sensei Desktop App

**Date:** 2026-05-04  
**Status:** Draft — pending implementation plan  
**Sections:** Bootstrap · Wizard · Observatory/Collective · Project Window · App Menu & Help

---

## 1. Executive Summary

The app transitions from a **single-window with a shared sidebar** to a **two-perspective model** using true Tauri multi-window:

| Window | Tauri Label | Purpose | Sidebar brand |
|---|---|---|---|
| Observatory (Collective) | `observatory` | Global view across all projects | 群 Collective |
| Project | `project-{id}` | Deep-dive into one project | Project kanji + name |

Clicking a project in the Observatory opens (or brings to front) its dedicated project window. Each perspective has its own layout group, sidebar, section set, and chrome style.

---

## 2. Window Architecture

### 2.1 Tauri Multi-Window

```
Main process
├── WebviewWindow "observatory"
│   URL: /observatory  (loads (observatory) SvelteKit layout)
│   Always present — never closes
│
└── WebviewWindow "project-{uuid}"   (one per open project)
    URL: /project/{id}  (loads (project) SvelteKit layout)
    Opened on demand, closed via window close button or ⌘W
```

**Opening a project window (TypeScript):**
```typescript
import { WebviewWindow } from '@tauri-apps/api/window';

export async function openProjectWindow(projectId: string) {
  const label = `project-${projectId.replace(/-/g, '')}`; // Tauri labels must be alphanumeric
  const existing = WebviewWindow.getByLabel(label);
  if (existing) { await existing.setFocus(); return; }
  new WebviewWindow(label, {
    url: `/project/${projectId}`,
    title: `Sensei · ${projectName}`,
    width: 1200, height: 820,
    minWidth: 900, minHeight: 600,
    decorations: false,   // we draw PerspectiveChrome
    transparent: false,
  });
}
```

**Window state store** (`$lib/stores/windows.svelte.ts`):
```typescript
interface OpenWindow { projectId: string; label: string; projectName: string }
let openProjectWindows = $state<Map<string, OpenWindow>>(new Map());
```

### 2.2 Chrome Distinction

| Window | Chrome component | Accent strip |
|---|---|---|
| Observatory | `TauriChrome` | none |
| Project | `PerspectiveChrome` | 2px `var(--shu)` top stripe |

`PerspectiveChrome` renders: traffic lights · centered `先生 · {project.name}` · `· project window` subtitle · top accent stripe.

---

## 3. Route Group Restructure

### 3.1 Current vs Target

**Current:**
```
(app)/+layout.svelte          ← single global sidebar
  observatory/
  sessions/
  learnings/
  libraries/
  instruments/
  projects/[id]/              ← project inside observatory layout
  settings/
```

**Target:**
```
(observatory)/+layout.svelte  ← Collective sidebar (renamed from Observatory)
  observatory/                ← Today / home
  projects/                   ← Projects index grid
  sessions/                   ← Global sessions (no project filter)
  insights/                   ← Learnings/Insights (renamed from learnings/)
  memories/                   ← Memory anatomy
  memories/sharing/
  memories/consolidate/
  upgrades/
  impact/                     ← Global impact
  libraries/                  ← Global libraries
  instruments/playground/
  instruments/replay/
  instruments/health/
  collective/                 ← Collective intel settings
  settings/                   ← Global settings (renamed from config)

(project)/+layout.svelte      ← Project sidebar
  project/[id]/+layout.svelte ← Loads project data for all children
  project/[id]/               ← default → redirect to /overview
  project/[id]/overview/
  project/[id]/sessions/
  project/[id]/memories/
  project/[id]/traceability/
  project/[id]/libraries/
  project/[id]/instruments/
  project/[id]/patterns/
  project/[id]/impact/
  project/[id]/about/

(config)/                     ← unchanged (setup wizard)
(health)/                     ← unchanged (logs, health)
```

### 3.2 Layout Responsibilities

**`(observatory)/+layout.svelte`:**
- Renders `CollectiveSidebar` (群 Collective header)
- Sidebar sections: Today · Projects · Sessions · Insights · Memories · Upgrades · Impact · Libraries · Instruments · Collective · Configure
- Active projects listed below with `↗ opens in its own window` affordance
- Clicking a project → calls `openProjectWindow(project.id)`
- Footer: daemon status

**`(project)/+layout.svelte`:**
- Renders `PerspectiveChrome` (drawn in-app since `decorations: false`)
- No outer drag region — the chrome IS the drag region
- Provides `ProjectSidebar` with sections list + health stats

**`(project)/project/[id]/+layout.svelte`:**
- Loads project data (name, kanji, FTR, repos count)
- Provides project context to all child pages via `$page.data` or a Svelte context
- Manages `activeSection` state (used by sidebar to highlight current route)

---

## 4. Observatory / Collective Sections

The Observatory window retains all current pages. Key changes:

| Section | Current route | New route | Change |
|---|---|---|---|
| Today | `/observatory` | `/observatory` | Rename sidebar label to "Collective" |
| Projects | sidebar item → `projects/` embedded | `/projects` | Dedicated full page (ProjectsIndexA) |
| Sessions | `/sessions` | `/sessions` | Add optional `?project={id}` filter support |
| Insights/Learnings | `/learnings` | `/insights` | Rename route |
| Memories | inline (no route) | `/memories` + sub-routes | Split into 3 sub-routes |
| Libraries | `/libraries` | `/libraries` | Add optional `?project={id}` filter |
| Instruments | `/instruments` | `/instruments/playground` etc. | Split into 3 sub-routes |
| Impact | new | `/impact` | New global impact page |
| Upgrades | new | `/upgrades` | New upgrades page |
| Collective | new | `/collective` | New collective intel page |
| Settings | `/settings` | `/settings` | Unchanged |

---

## 5. Shared Components with Project Filter

Three components work in both windows; they accept an optional `projectFilter` prop:

### 5.1 `SessionsList`
**File:** `$lib/components/sessions/SessionsList.svelte`
```typescript
interface Props {
  projectFilter?: string;   // project UUID — if set, scopes to that project
  limit?: number;
  showProjectColumn?: boolean;  // false in project window (redundant)
}
```
- Observatory Sessions page: `<SessionsList/>` (no filter)  
- Project Sessions page: `<SessionsList projectFilter={projectId} showProjectColumn={false}/>`

### 5.2 `LibrariesList`
**File:** `$lib/components/libraries/LibrariesList.svelte`
```typescript
interface Props {
  projectFilter?: string;   // if set, uses libraries_in_project view
  showUsageStats?: boolean; // show calls/usage in project context
}
```

### 5.3 `InstrumentsList`  
**File:** `$lib/components/instruments/InstrumentsList.svelte`
```typescript
interface Props {
  projectFilter?: string;   // if set, shows extensions scoped to that project
  scope?: 'global' | 'project' | 'all';
}
```

---

## 6. Project Window — Pages, State, Stubs, Load Functions

The project window has 9 sections. Each section is a route that receives `project` from the parent layout via Svelte context.

### 6.0 Shared project data (parent layout loads)

```typescript
// project/[id]/+layout.ts
export async function load({ params, fetch }) {
  const project = await fetch(`/api/projects/${params.id}`).then(r => r.json());
  const ftrMetrics = await fetch(`/api/projects/${params.id}/ftr`).then(r => r.json());
  return { project, ftrMetrics };
}

interface ProjectLayoutData {
  project: {
    id: string; name: string; client?: string; goal?: string;
    maturity: string; stack: Stack; icon: Icon; preferred_acp?: string;
    kanji: string;  // derived from icon.value
  };
  ftrMetrics: {
    ftr14d: number;       // 0–1
    ftr14dPrev: number;   // prior 14d for delta
    ftrTrend: number[];   // 14 values, one per day
    sessions7d: number;
    driftCount: number;   // from drift_items joined through folders
  };
}
```

---

### 6.1 Overview

**State:**
```typescript
interface OverviewPageData extends ProjectLayoutData {
  repos: Folder[];           // folders where project_id = id
  topRecommendation: Recommendation | null;
  memoryCount: number;
  memoriesPendingShare: number;
  recentSessions: Session[];  // last 4
}
```

**Load function:**
```typescript
export async function load({ params, fetch, parent }) {
  const { project, ftrMetrics } = await parent();
  const [repos, recs, memories, sessions] = await Promise.all([
    fetch(`/api/projects/${params.id}/repos`).then(r => r.json()),
    fetch(`/api/projects/${params.id}/recommendations?status=pending&limit=1`).then(r => r.json()),
    fetch(`/api/projects/${params.id}/memories?status=active&count=true`).then(r => r.json()),
    fetch(`/api/sessions?project_id=${params.id}&limit=4`).then(r => r.json()),
  ]);
  return { project, ftrMetrics, repos, topRecommendation: recs[0] ?? null,
           memoryCount: memories.total, memoriesPendingShare: memories.pendingShare,
           recentSessions: sessions.sessions };
}
```

**Stub component:** `ProjOverviewLite` (mockup reference) — renders project header, hero recommendation card, 3 stat blocks, recent sessions list.

---

### 6.2 Sessions

**State:**
```typescript
interface SessionsPageData {
  projectId: string;
  // rest loaded client-side via SessionsList component with projectFilter
}
```

**Implementation:** Wrap `<SessionsList projectFilter={projectId} showProjectColumn={false}/>`. Sessions page is thin — the shared component does the work.

---

### 6.3 Memories

**State:**
```typescript
interface MemoriesPageData extends ProjectLayoutData {
  memories: Memory[];
  pendingShare: Memory[];
}

interface Memory {
  id: string; title: string; type: string; scope: string;
  status: string; strength: number; reinforced_count: number;
  violated_count: number; last_relevant_at: string;
  session_id?: string;
}
```

**Load:**
```typescript
export async function load({ params, fetch, parent }) {
  const { project } = await parent();
  const data = await fetch(`/api/projects/${params.id}/memories`).then(r => r.json());
  return { project, memories: data.active, pendingShare: data.pending_share };
}
```

**Stub component:** `ProjMemoriesLite` (mockup reference) — hero card for pending-share batch, list of active memories with status badges.

---

### 6.4 Traceability

**State:**
```typescript
interface TraceabilityPageData extends ProjectLayoutData {
  driftItems: DriftItem[];
  totalTracked: number;
  driftedCount: number;
  brokenCount: number;
}

interface DriftItem {
  id: string;
  docNode: { file_path: string; name: string; content?: string };
  codeNode: { file_path: string; name: string; signature?: string } | null;
  status: 'current' | 'drifted' | 'broken';
  expectedSignature?: string;
  actualSignature?: string;
  detail?: string;
  detectedAt: string;
}
```

**Load:**
```typescript
export async function load({ params, fetch, parent }) {
  const { project } = await parent();
  const data = await fetch(`/api/projects/${params.id}/drift`).then(r => r.json());
  return { project, driftItems: data.items, totalTracked: data.total,
           driftedCount: data.drifted, brokenCount: data.broken };
}
```

**Stub component:** `ProjTraceabilityLite` (mockup reference) — hero for worst drift, list of drifted docs with fix-drift prompt action.

---

### 6.5 Libraries

**State:**
```typescript
interface LibrariesPageData extends ProjectLayoutData {
  libraries: ProjectLibrary[];
  wrappedCount: number;
  unwrappedCount: number;
}

interface ProjectLibrary {
  library_id: string; name: string; ecosystem: string;
  description?: string; version_used?: string;
  usageCount: number;      // from referenced_libraries.props.usage_count
  hasInstruments: boolean; // derived: props.skill_path != null
}
```

**Load:** Uses `libraries_in_project` view (already exists).  
```typescript
export async function load({ params, fetch, parent }) {
  const { project } = await parent();
  const data = await fetch(`/api/projects/${params.id}/libraries`).then(r => r.json());
  return { project, libraries: data.libraries,
           wrappedCount: data.libraries.filter((l: any) => l.hasInstruments).length,
           unwrappedCount: data.libraries.filter((l: any) => !l.hasInstruments).length };
}
```

**Stub component:** `ProjLibrariesLite` (mockup reference) — hero for library coverage, list with wrapped/unwrapped status.

---

### 6.6 Instruments

**State:**
```typescript
interface InstrumentsPageData extends ProjectLayoutData {
  tools: ProjectTool[];
  totalCalls7d: number;
  avgFtr: number;
}

interface ProjectTool {
  id: string; name: string; description?: string;
  scope: string;   // "global" | "project" | "folder"
  calls7d: number; ftr: number;  // from activity.task_sessions or props
  libraryName?: string;   // which library this instrument wraps
}
```

**Load:** `GET /api/projects/{id}/instruments` — extensions where scope='project' OR extensions that wrap libraries in this project.

**Stub component:** `ProjInstrumentsLite` (mockup reference) — hero for weakest tool, list with FTR per tool.

---

### 6.7 Patterns

**State:**
```typescript
interface PatternsPageData extends ProjectLayoutData {
  followed: DetectedPattern[];
  antiPatterns: DetectedPattern[];
}

interface DetectedPattern {
  id: string; name: string; family?: string;
  lifecycle: string;         // suggested | gap | rule
  is_anti_pattern: boolean;
  severity?: string;
  confidence: number;
  instance_count: number;
  description?: string;
  example?: string;
  enforcement?: string;
  fix_pattern_id?: string;
}
```

**Load:** `GET /api/projects/{id}/patterns` — joins `detected_patterns` through `folders`.

**Stub component:** `ProjPatterns` (mockup reference, already in project-shared.jsx) — toggle follow/anti, list + detail pane.

---

### 6.8 Impact

**State:**
```typescript
interface ImpactPageData extends ProjectLayoutData {
  verdicts: Recommendation[];
  positiveCount: number;
  negativeCount: number;
  pendingCount: number;
}

interface Recommendation {
  id: string; title: string; why: string; impact?: string;
  urgency: string; status: string;
  verdict: 'positive' | 'negative' | 'neutral' | 'pending';
  baselineFtr?: number; currentFtr?: number;
  actedAt?: string; measuredAt?: string;
  defaultAcp?: string; prompt?: string;
  evidence: Array<{ session_id: string; file?: string; description: string }>;
}
```

**Load:**
```typescript
export async function load({ params, fetch, parent }) {
  const { project } = await parent();
  const data = await fetch(`/api/projects/${params.id}/recommendations?status=accepted`).then(r => r.json());
  return { project, verdicts: data,
           positiveCount: data.filter((r: any) => r.verdict === 'positive').length,
           negativeCount: data.filter((r: any) => r.verdict === 'negative').length,
           pendingCount:  data.filter((r: any) => r.verdict === 'pending').length };
}
```

**Stub component:** `ProjImpactLite` (mockup reference) — hero win summary, verdict list with before/after FTR.

---

### 6.9 About (Project Settings)

**State:**
```typescript
interface AboutPageData extends ProjectLayoutData {
  repos: Folder[];
  settings: {
    links: ProjectLink[];
    guidelines: ProjectGuideline[];
    backlog: BacklogItem[];
    skills: { id: string; name: string; on: boolean }[];
    excluded: string[];
    privacy: ProjectPrivacy;
  }
}
```

**Load:** Full project object (already has links, guidelines, backlog in JSONB); repos separately.

**Stub component:** `ProjAboutPane` (mockup reference) — read-only by default, Edit toggle activates inline editing via `ProjSettingsV2`.

---

## 7. DB Gap Analysis

### 7.1 Missing: Project-scoped FTR metrics

The `projects` table has no computed FTR. Need a view or daemon endpoint:

```sql
-- ADD TO: daemon/database/ddl/view/sensei/project_ftr_metrics.ddl
CREATE OR REPLACE VIEW sensei.project_ftr_metrics AS
WITH daily AS (
  SELECT project_id,
         date_trunc('day', started_at) AS day,
         AVG(CASE WHEN ftr THEN 1.0 ELSE 0.0 END) AS daily_ftr
    FROM activity.sessions
   WHERE project_id IS NOT NULL
     AND started_at > now() - interval '28d'
   GROUP BY project_id, day
)
SELECT
  s.project_id,
  COUNT(*) FILTER (WHERE started_at > now() - interval '7d')  AS sessions_7d,
  AVG(CASE WHEN ftr THEN 1.0 ELSE 0.0 END)
    FILTER (WHERE started_at > now() - interval '14d')        AS ftr_14d,
  AVG(CASE WHEN ftr THEN 1.0 ELSE 0.0 END)
    FILTER (WHERE started_at > now() - interval '28d'
              AND started_at <= now() - interval '14d')       AS ftr_14d_prev
FROM activity.sessions s
WHERE project_id IS NOT NULL
GROUP BY s.project_id;
```

For the 14-day trend array (needed by FTR strip chart) — return last 14 `daily_ftr` values ordered by day from the daemon endpoint, not the view (avoid PostgreSQL array aggregation complexity).

### 7.2 Missing: Drift items scoped to project

`drift_items` has `folder_id` but no `project_id`. Two options:

**Option A (view — preferred):**
```sql
-- ADD TO: daemon/database/ddl/view/sensei/project_drift.ddl
CREATE OR REPLACE VIEW sensei.project_drift AS
SELECT di.*, f.project_id
  FROM inference.drift_items di
  JOIN sensei.folders f ON f.id = di.folder_id
 WHERE f.project_id IS NOT NULL;
```

**Option B (denormalize):** Add `project_id` to `drift_items` as a nullable FK. Populated on insert by joining through folder. Better for index performance but increases write complexity.

**Decision: Option A first** (simpler DDL change, no migration risk). If performance is a concern after profiling, denormalize.

### 7.3 Missing: Patterns scoped to project

Same structure as drift — `detected_patterns` is by `folder_id`:

```sql
-- ADD TO: daemon/database/ddl/view/sensei/project_patterns.ddl
CREATE OR REPLACE VIEW sensei.project_patterns AS
SELECT dp.*, f.project_id
  FROM inference.detected_patterns dp
  JOIN sensei.folders f ON f.id = dp.folder_id
 WHERE f.project_id IS NOT NULL;
```

### 7.4 Unified bridge table pattern — libraries, extensions, instruments

All three relationships between projects and their associated entities follow the same design:

> **`project_id = NULL` → global (available to every project)**  
> **`project_id = X` → scoped to that project only**  
> **No rows → inactive / not yet associated**

This gives you one consistent query shape across all three: `WHERE project_id = $1 OR project_id IS NULL`.

---

#### 7.4a `project_libraries` bridge

The existing `referenced_libraries` table operates at the **folder level** (auto-detected from `package.json`, `Cargo.toml`, etc.) and does not model project-level or global availability. A new project-level bridge sits above it:

```sql
-- ADD TO: daemon/database/ddl/table/sensei/project_libraries.ddl
CREATE TABLE IF NOT EXISTS sensei.project_libraries (
  library_id   uuid         NOT NULL REFERENCES sensei.libraries(id) ON DELETE CASCADE,
  project_id   uuid         REFERENCES sensei.projects(id) ON DELETE CASCADE,  -- NULL = global
  enabled      boolean      NOT NULL DEFAULT true,
  props        jsonb        NOT NULL DEFAULT '{}',
  modified_at  timestamptz  NOT NULL DEFAULT now(),
  PRIMARY KEY (library_id, COALESCE(project_id, '00000000-0000-0000-0000-000000000000'))
);

CREATE INDEX IF NOT EXISTS project_libraries_project_id_idx
    ON sensei.project_libraries(project_id)
 WHERE enabled;

COMMENT ON TABLE sensei.project_libraries IS
'Many-to-many: libraries associated with projects or marked global.
- project_id NULL  = global (available in every project)
- project_id = X   = scoped to that project
Auto-detected usage is in referenced_libraries (folder-level).
This table is the curated/explicit layer — populated by the daemon when a library
is confirmed in a project, and editable by the user.';
```

> **Note on PK with nullable column:** PostgreSQL treats each NULL as distinct, so `PRIMARY KEY (library_id, project_id)` won't deduplicate the global row. Use a partial unique index instead of a composite PK:
> ```sql
> -- Replace the PRIMARY KEY above with:
> CREATE UNIQUE INDEX project_libraries_global_uniq
>     ON sensei.project_libraries(library_id)
>  WHERE project_id IS NULL;
> CREATE UNIQUE INDEX project_libraries_scoped_uniq
>     ON sensei.project_libraries(library_id, project_id)
>  WHERE project_id IS NOT NULL;
> ```

**Query — libraries for project X:**
```sql
SELECT l.* FROM sensei.libraries l
JOIN sensei.project_libraries pl ON pl.library_id = l.id
WHERE (pl.project_id = $1 OR pl.project_id IS NULL)
  AND pl.enabled = true;
```

The `libraries_in_project` view is updated to read from `project_libraries` (not `referenced_libraries`). The daemon's indexer populates `project_libraries` from `referenced_libraries` when it confirms library usage in a folder that belongs to a project.

---

#### 7.4b `extension_projects` bridge

Same pattern for extensions (skills, commands, agents, hooks, instruments):

```sql
-- ADD TO: daemon/database/ddl/table/sensei/extension_projects.ddl
CREATE TABLE IF NOT EXISTS sensei.extension_projects (
  extension_id  uuid         NOT NULL REFERENCES sensei.extensions(id) ON DELETE CASCADE,
  project_id    uuid         REFERENCES sensei.projects(id) ON DELETE CASCADE,  -- NULL = global
  enabled       boolean      NOT NULL DEFAULT true,
  props         jsonb        NOT NULL DEFAULT '{}',   -- per-project config overrides
  modified_at   timestamptz  NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX extension_projects_global_uniq
    ON sensei.extension_projects(extension_id)
 WHERE project_id IS NULL;

CREATE UNIQUE INDEX extension_projects_scoped_uniq
    ON sensei.extension_projects(extension_id, project_id)
 WHERE project_id IS NOT NULL;

CREATE INDEX extension_projects_project_id_idx
    ON sensei.extension_projects(project_id)
 WHERE enabled AND project_id IS NOT NULL;

COMMENT ON TABLE sensei.extension_projects IS
'Many-to-many: extensions (skills/commands/agents/hooks) associated with projects or global.
- project_id NULL  = global (active in every project)
- project_id = X   = active only in that project
- enabled = false  = explicitly disabled (even if a global row exists)
- props            = per-project overrides (custom prompts, scoped paths, etc.)
The extensions.scope column becomes a UI hint only; this table is authoritative.';
```

**Query — extensions for project X:**
```sql
SELECT e.*, ep.props as project_props
FROM sensei.extensions e
JOIN sensei.extension_projects ep ON ep.extension_id = e.id
WHERE (ep.project_id = $1 OR ep.project_id IS NULL)
  AND ep.enabled = true
ORDER BY ep.project_id NULLS LAST;  -- project-specific rows rank above global
```

**Instruments** are extensions with `kind IN ('skill', 'command', 'agent')` that wrap a library tool. They use the same `extension_projects` bridge — no separate instruments table needed.

---

#### 7.4c `extensions.scope` column

The `scope` enum on `extensions` becomes a **UI default / seeding hint**, not an access-control gate:
- `global` → daemon seeds a `project_id = NULL` row in `extension_projects` on install
- `project` → requires explicit `extension_projects` rows (no auto-seed)
- `folder` → future: folder-level bridge (out of scope for this design)

The bridge tables are the runtime source of truth.

---

#### 7.4d Project-scoped views with scope tag

The UI must display whether each library, extension, or instrument is **global** (available everywhere) or **project-specific** (explicitly associated). A `scope` derived column ('global' | 'project') drives the badge in the UI.

**`project_libraries_resolved` view** — libraries visible to a given project, with scope tag:

```sql
-- ADD TO: daemon/database/ddl/view/sensei/project_libraries_resolved.ddl
CREATE OR REPLACE VIEW sensei.project_libraries_resolved AS
SELECT
    l.*,
    pl.enabled,
    pl.props          AS project_props,
    pl.modified_at    AS associated_at,
    CASE
        WHEN pl.project_id IS NULL THEN 'global'
        ELSE 'project'
    END               AS scope,
    pl.project_id     AS scoped_project_id
FROM sensei.libraries l
JOIN sensei.project_libraries pl ON pl.library_id = l.id;
```

Query for a specific project (returns global + project-scoped rows):
```sql
SELECT * FROM sensei.project_libraries_resolved
WHERE (scoped_project_id = $1 OR scoped_project_id IS NULL)
  AND enabled = true
ORDER BY scope DESC, name;  -- 'project' sorts before 'global'
```

---

**`project_extensions_resolved` view** — extensions/instruments visible to a given project, with scope tag:

```sql
-- ADD TO: daemon/database/ddl/view/sensei/project_extensions_resolved.ddl
CREATE OR REPLACE VIEW sensei.project_extensions_resolved AS
SELECT
    e.*,
    ep.enabled,
    ep.props          AS project_props,
    ep.modified_at    AS associated_at,
    CASE
        WHEN ep.project_id IS NULL THEN 'global'
        ELSE 'project'
    END               AS scope,
    ep.project_id     AS scoped_project_id
FROM sensei.extensions e
JOIN sensei.extension_projects ep ON ep.extension_id = e.id;
```

Query for a specific project:
```sql
SELECT * FROM sensei.project_extensions_resolved
WHERE (scoped_project_id = $1 OR scoped_project_id IS NULL)
  AND enabled = true
ORDER BY scope DESC, name;
```

**Instruments** are queried by adding `AND kind IN ('skill', 'command', 'agent')`.

---

**TypeScript type for the scope tag** (used in both Libraries and Instruments pages):
```typescript
type ScopeTag = 'global' | 'project';

interface ProjectLibrary {
  id: string;
  name: string;
  version?: string;
  scope: ScopeTag;        // drives the [global] / [project] badge
  enabled: boolean;
  projectProps: Record<string, unknown>;
}

interface ProjectExtension {
  id: string;
  name: string;
  kind: string;
  scope: ScopeTag;        // drives the [global] / [project] badge
  enabled: boolean;
  projectProps: Record<string, unknown>;
}
```

### 7.5 Nodes/edges — graph scoped to project

`nodes` and `edges` are by `folder_id`. The project graph view needs to aggregate across all folders in a project. This is a query-time join — no new DDL needed, but the daemon needs a project graph endpoint:

```
GET /api/projects/{id}/graph
→ SELECT n.*, f.project_id FROM nodes n JOIN folders f ON f.id = n.folder_id WHERE f.project_id = ?
→ SELECT e.* FROM edges e JOIN folders f ON f.id = e.folder_id WHERE f.project_id = ?
```

For large projects this should be paginated or summarized (return top N nodes by degree, all edges between them).

### 7.6 Summary of DDL changes

| Change | File | Type |
|---|---|---|
| `project_ftr_metrics` view | `view/sensei/project_ftr_metrics.ddl` | New view |
| `project_drift` view | `view/sensei/project_drift.ddl` | New view |
| `project_patterns` view | `view/sensei/project_patterns.ddl` | New view |
| `project_libraries` bridge table | `table/sensei/project_libraries.ddl` | New table (many-to-many lib↔project) |
| `extension_projects` bridge table | `table/sensei/extension_projects.ddl` | New table (many-to-many ext↔project) |
| `project_libraries_resolved` view | `view/sensei/project_libraries_resolved.ddl` | New view — adds `scope` tag |
| `project_extensions_resolved` view | `view/sensei/project_extensions_resolved.ddl` | New view — adds `scope` tag |

---

## 8. Daemon API Changes

New endpoints needed (all under `/api/`):

| Endpoint | Returns | Source |
|---|---|---|
| `GET /projects/{id}/ftr` | `{ ftr14d, ftr14dPrev, ftrTrend[14], sessions7d }` | `project_ftr_metrics` view + daily aggregation |
| `GET /projects/{id}/repos` | `Folder[]` | `folders WHERE project_id = ?` |
| `GET /projects/{id}/drift` | `{ items: DriftItem[], total, drifted, broken }` | `project_drift` view |
| `GET /projects/{id}/patterns` | `{ followed: Pattern[], antiPatterns: Pattern[] }` | `project_patterns` view |
| `GET /projects/{id}/libraries` | `{ libraries: ProjectLibrary[] }` with `scope: 'global'\|'project'` per row | `project_libraries_resolved` view |
| `GET /projects/{id}/instruments` | `{ tools: ProjectExtension[] }` with `scope: 'global'\|'project'` per row | `project_extensions_resolved` view (kind filter) |
| `GET /projects/{id}/recommendations` | `Recommendation[]` | existing FK, filter by `?status=` |
| `GET /projects/{id}/memories` | `{ active: Memory[], pending_share: Memory[], total, pendingShare }` | existing FK |
| `GET /projects/{id}/graph` | `{ nodes, edges }` (paginated) | join through folders |

**Existing endpoints that need a `?project_id=` filter added:**
- `GET /sessions` — add optional `project_id` query param (already has the index)

---

## 9. App Menu & Help System

Mac-native menu via Tauri's `tauri::menu::Menu`:

```
Sensei
  About Sensei
  ──
  Preferences…     ⌘,
  ──
  Hide Sensei      ⌘H
  Quit Sensei      ⌘Q

File
  New Project       ⌘N
  ──
  Close Window      ⌘W

Edit
  Undo              ⌘Z
  Redo             ⌘⇧Z
  ──
  Cut               ⌘X
  Copy              ⌘C
  Paste             ⌘V
  Select All        ⌘A

View
  Toggle Sidebar    ⌘\
  ──
  Today             ⌘1   (Observatory only)
  Projects          ⌘2
  Sessions          ⌘3
  Libraries         ⌘4
  Instruments       ⌘5

Window
  Show Observatory  ⌘0
  Minimize          ⌘M
  Zoom
  ──
  [dynamic: open project windows listed here]
  ──
  Bring All to Front

Help
  Sensei Help           ?
  Keyboard Shortcuts    ⌘/
  What's New
  ──
  Report an Issue
  About Sensei
```

### 9.1 Help window / panel

The Help system is a lightweight in-app panel (not a browser), implemented as a Tauri window (`help` label) loading `/help`:
- Keyboard shortcuts reference
- Quick-start guide (3 steps: scan → watch → teach)
- FAQ: daemon, memory scope, FTR definition
- Release notes (`What's New`)

Panel is searchable. Opens at `⌘?` or from menu. Stays open alongside other windows.

---

## 10. Backlog — Checklisted by Section

### BOOTSTRAP (existing, no major changes)
- [ ] Verify bootstrap flow works with new route structure (no route changes, just ensure `/` redirect still works)
- [ ] Ensure bootstrap step detection (`scan_state`) is unaffected by layout group rename

---

### WIZARD / SETUP (config group — unchanged)
- [ ] Verify all setup routes (`/setup/*`) still work after layout group rename
- [ ] Ensure wizard "Done" → redirects to `/observatory` (update redirect target if needed)

---

### OBSERVATORY — COLLECTIVE (primary window)

**Layout & Navigation**
- [ ] Rename `(app)` route group to `(observatory)`
- [ ] Update sidebar label from "Observatory" to "Collective" (群 kanji)
- [ ] Add "↗ opens in its own window" affordance to project list items
- [ ] Wire project list click → `openProjectWindow(project.id)` Tauri call
- [ ] Add "Dormant" section to sidebar project list (projects not active in 7d)
- [ ] Update footer to show "daemon · running / last heartbeat Xs ago" (live from SSE)

**Today / Home Page** (`/observatory`)
- [ ] Replace current Observatory page with full `ObsHome` layout (hero koan, insights, adopted, recent sessions)
- [ ] Wire FTR 14d strip chart with real data from `project_ftr_metrics` view
- [ ] Wire hero koan to top pending recommendation (highest urgency across all projects)
- [ ] Wire "Also worth noticing" insights list to `insight_batches`
- [ ] Wire "System has learned" adopted teachings list to memories with status=reinforced
- [ ] Wire recent sessions strip to `/api/sessions?limit=4`
- [ ] Handle early/empty state (< 5 sessions watched — "Still listening" koan)

**Projects Index** (`/projects`)
- [ ] Create `/projects` full-page route (currently embedded in sidebar)
- [ ] Implement `ProjectsIndexA` grid with status filters (All / Active / Dormant / Archived)
- [ ] Wire project card click → `openProjectWindow(id)`
- [ ] Add FTR and session stats to each project card (from `project_ftr_metrics`)
- [ ] Support search by project name or client

**Sessions** (`/sessions`)
- [ ] Extract current sessions page into `SessionsList` shared component
- [ ] Add `projectFilter?: string` prop to `SessionsList`
- [ ] Wire `SessionsDigestZen` mockup layout (vs. current simple list)
- [ ] Add project column (shown only in Observatory context)

**Insights** (`/insights`)
- [ ] Create `/insights` route (rename from `/learnings`)
- [ ] Wire `LearningsTriage` component with real data from `insight_batches` + `insights` tables
- [ ] Update sidebar link from "Learnings" to "Insights"

**Memories** (`/memories`, `/memories/sharing`, `/memories/consolidate`)
- [ ] Create 3 sub-routes for Memories sub-sections
- [ ] Wire Anatomy page with real memories data
- [ ] Wire Sharing page with memories pending collective share
- [ ] Wire Consolidate page with memories pending consolidation

**Libraries** (`/libraries`)
- [ ] Add `projectFilter` support to LibrariesList component
- [ ] No route change needed; context is "global" (no filter)

**Instruments** (`/instruments/playground`, `/instruments/replay`, `/instruments/health`)
- [ ] Split instruments into 3 sub-routes (currently one `/instruments` page)
- [ ] Wire Playground (test tool calls), Replay (session replay), Health (FTR per tool) with real data
- [ ] Add `projectFilter` support to InstrumentsList component

**Impact** (`/impact`)
- [ ] Create new `/impact` page (global across all projects)
- [ ] Wire `ObsImpact` component with recommendations where verdict != pending

**Upgrades** (`/upgrades`)
- [ ] Create new `/upgrades` page
- [ ] Wire `ObsUpgrades` component (new model/skill upgrades available)

**Collective** (`/collective`)
- [ ] Create new `/collective` page
- [ ] Wire `ObsCollectiveSettings` component (collective intel settings)

---

### PROJECT WINDOW

**Architecture**
- [ ] Add `WebviewWindow` open/focus logic in `$lib/stores/windows.svelte.ts`
- [ ] Create `(project)` route group with `PerspectiveChrome` layout
- [ ] Create `project/[id]/+layout.svelte` — loads project + FTR metrics, provides to children via Svelte context
- [ ] Implement `ProjectSidebarRouted` — sidebar drives section routing (links to `/project/{id}/section`)
- [ ] Add "⇆ switch project" button logic (opens project picker or navigates back to Observatory)
- [ ] Wire health stats in sidebar (FTR 14d, Sessions 7d, Drift watch count) from layout data

**Overview** (`/project/[id]/overview`)
- [ ] Create stub with `ProjOverviewLite` structure
- [ ] Wire project header (kanji, name, client, FTR, FTR trend sparkline)
- [ ] Wire hero recommendation card (top pending recommendation)
- [ ] Wire stat blocks (Sessions 7d, Memories, Doc drift count)
- [ ] Wire recent sessions list
- [ ] Load function: fetch repos, top recommendation, memory count, recent sessions

**Sessions** (`/project/[id]/sessions`)
- [ ] Wrap `SessionsList projectFilter={projectId} showProjectColumn={false}`
- [ ] Add session-specific stats strip (FTR 7d, corrections, first-try count for this project)
- [ ] Load function: project context only (sessions loaded client-side by SessionsList)

**Memories** (`/project/[id]/memories`)
- [ ] Create `ProjMemoriesLite` Svelte implementation
- [ ] Wire pending-share hero card
- [ ] Wire active memories list with strength indicator and status badges
- [ ] Load function: `GET /api/projects/{id}/memories`
- [ ] **New daemon endpoint**: `GET /api/projects/{id}/memories`

**Traceability** (`/project/[id]/traceability`)
- [ ] Create `ProjTraceabilityLite` Svelte implementation
- [ ] Wire drift items list with doc path, symbol path, severity, fix-drift action
- [ ] Wire hero card to worst drift item
- [ ] Load function: `GET /api/projects/{id}/drift`
- [ ] **New DB view**: `sensei.project_drift`
- [ ] **New daemon endpoint**: `GET /api/projects/{id}/drift`

**Libraries** (`/project/[id]/libraries`)
- [ ] Create `ProjLibrariesLite` Svelte implementation
- [ ] Wire library list with `scope` badge (`[global]` / `[project]`) from `project_libraries_resolved` view
- [ ] Wire "wrap library" action (adds instrument for a library)
- [ ] Load function: `GET /api/projects/{id}/libraries` (uses `project_libraries_resolved` view)
- [ ] **New DB table**: `sensei.project_libraries` (see §7.4a)
- [ ] **New DB view**: `sensei.project_libraries_resolved` (adds scope tag, see §7.4d)

**Instruments** (`/project/[id]/instruments`)
- [ ] Create `ProjInstrumentsLite` Svelte implementation
- [ ] Wire tool list with `scope` badge (`[global]` / `[project]`) from `project_extensions_resolved` view
- [ ] Wire "open replay" action for weak tool
- [ ] Load function: `GET /api/projects/{id}/instruments`
- [ ] **New DB table**: `sensei.extension_projects` (many-to-many bridge, see §7.4b)
- [ ] **New DB view**: `sensei.project_extensions_resolved` (adds scope tag, see §7.4d)
- [ ] **New daemon endpoint**: `GET /api/projects/{id}/instruments`

**Patterns** (`/project/[id]/patterns`)
- [ ] Implement `ProjPatterns` Svelte (follow/avoid toggle, list + detail pane)
- [ ] Wire followed patterns list (lifecycle: suggested/gap/rule)
- [ ] Wire anti-patterns list with severity + suggested fix cross-link
- [ ] Wire "Adopt pattern" and "Promote pattern" actions → `ProjActionDrawer`
- [ ] Implement `ProjActionDrawer` Svelte (ACP picker + prompt editor)
- [ ] Load function: `GET /api/projects/{id}/patterns`
- [ ] **New DB view**: `sensei.project_patterns`
- [ ] **New daemon endpoint**: `GET /api/projects/{id}/patterns`

**Impact** (`/project/[id]/impact`)
- [ ] Create `ProjImpactLite` Svelte implementation
- [ ] Wire verdict list with before/after FTR comparison
- [ ] Wire pending/positive/negative counts in hero card
- [ ] Load function: `GET /api/projects/{id}/recommendations?status=accepted`

**About** (`/project/[id]/about`)
- [ ] Implement `ProjAboutPane` Svelte (read-mode by default, Edit toggle)
- [ ] Wire `ProjSettingsV2` document-style settings inside About pane
- [ ] Inline editing: name, client, goal, icon, stack, repos, links, guidelines, backlog
- [ ] `PATCH /api/projects/{id}` on save
- [ ] Load function: full project + repos

---

### DB / SCHEMA

- [ ] Add `daemon/database/ddl/table/sensei/project_libraries.ddl` — new bridge table (many-to-many lib↔project)
- [ ] Add `daemon/database/ddl/table/sensei/extension_projects.ddl` — new bridge table (many-to-many ext↔project)
- [ ] Add view `daemon/database/ddl/view/sensei/project_ftr_metrics.ddl`
- [ ] Add view `daemon/database/ddl/view/sensei/project_drift.ddl`
- [ ] Add view `daemon/database/ddl/view/sensei/project_patterns.ddl`
- [ ] Add view `daemon/database/ddl/view/sensei/project_libraries_resolved.ddl` — joins libraries + bridge, emits `scope` column
- [ ] Add view `daemon/database/ddl/view/sensei/project_extensions_resolved.ddl` — joins extensions + bridge, emits `scope` column
- [ ] Reset and re-apply schema (`dbd` reset + apply)

---

### DAEMON API

- [ ] `GET /api/projects/{id}/ftr` — FTR metrics with 14-day trend array
- [ ] `GET /api/projects/{id}/repos` — folders where project_id = id
- [ ] `GET /api/projects/{id}/drift` — drift items via project_drift view
- [ ] `GET /api/projects/{id}/patterns` — patterns via project_patterns view
- [ ] `GET /api/projects/{id}/instruments` — extensions scoped to this project
- [ ] `GET /api/projects/{id}/memories` — active memories + pending share count
- [ ] Add `?project_id=` filter to `GET /api/sessions`

---

### APP MENU & HELP

- [ ] Register native Tauri menu (`tauri::menu::Menu`) in `src-tauri/src/main.rs`
- [ ] Wire standard Edit menu items (Undo/Redo/Cut/Copy/Paste/SelectAll) via system defaults
- [ ] Wire View → Toggle Sidebar (emit event to frontend)
- [ ] Wire View → section shortcuts (⌘1–5, emit section-change event)
- [ ] Wire Window → Show Observatory (focus observatory window)
- [ ] Wire Window → dynamic project window list (update menu on window open/close)
- [ ] Wire Help → Keyboard Shortcuts (⌘/) → open `/help` window
- [ ] Create `/help` route with keyboard shortcuts reference and quick-start guide
- [ ] Wire Help → Report an Issue (open GitHub issues URL in browser)

---

## 11. Open Questions / Future

- **Graph page** — `project/[id]/overview` contains the code graph (nodes+edges). Deferred until indexer produces reliable data. Stub with placeholder.
- **Project window graph section** — `ProjGraphLens` is not in the new project sidebar (replaced by Overview section showing stats). The dedicated graph tab becomes part of Overview or a future section.
- **Collective intelligence** — `ObsConsolidation`, `ObsSharingReview` are complex workflows. Initial impl can be stubs that show counts.
- **Tauri window persistence** — OS-level window position/size should persist per project window using Tauri `window-state` plugin.
- **Multiple project windows** — max N open at once? Currently unconstrained. Consider showing a warning at 5+.
