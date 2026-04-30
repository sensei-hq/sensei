# Wizard State Architecture

> Canonical pattern for setup wizard data flow: load, state, commit, re-entry.

**Date:** 2026-04-30
**Status:** Design
**Scope:** `app/src/lib/wizard-state.svelte.ts`, layout load pipeline, stage commit flow

---

## Problem

The setup wizard has 11 stages. Currently 4 patterns coexist: static pages, SvelteKit load + local state, ReactiveStageContext + SSE, and placeholders. User selections in Assistants and Folders are never persisted back to the daemon. There is no shared state across stages, no re-entry support, and no stage-completion tracking.

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| State pattern | Thin singleton (`WizardState`) | Matches existing `appState` / `bootstrapState` pattern |
| Hydration | Layout load fetches all data from daemon in parallel | One load, all stages hydrated at once |
| Persistence | Save on advance — `commitStage()` POSTs to daemon then updates config key | Clean contract: advancing = committing |
| Stage completion | Config key-value store (`setup.welcome=done`, `setup.roots=done`, ...) | Fast lookup, no complex queries |
| Scan state | Standalone — `ScanProjectState` + `ScanActivityState` + `EventManager` | Streaming is fundamentally different from load-once stages |
| Downstream stages | Read from daemon after scan, not from scan state | Projects/Libraries/Instruments load real DB data, not replayed events |
| Re-entry | Skip to first pending stage, all data preloaded, free backward navigation | User can revisit any completed stage and re-commit |
| Naming | `folders` entity stays internally, UI shows kind-appropriate label (Repository/Folder). Watch roots stage labeled "Roots" | Avoids overloading "folder" in UI while keeping DB stable |

## Architecture

### WizardState Singleton

Lives at `app/src/lib/wizard-state.svelte.ts`. Exported as `wizardState`.

```typescript
class WizardState {
  // ── Completion tracking (from daemon config keys) ──
  completion = $state<Record<string, 'pending' | 'done'>>({});

  // ── Per-stage slices ──
  preferences = $state<PreferencesSlice>({
    displayName: '', contributeLearnings: true, reviewBeforeShare: true,
    shareSchedule: 'weekly-saturday', correctionAggressiveness: 'balanced',
    digestCadence: 'daily', nudgeOnRegression: true, anonymizedTelemetry: false,
  });
  assistants  = $state<AssistantsSlice>({ families: [], selected: {} });
  roots       = $state<RootsSlice>({ roots: [], newPath: '' });
  scan        = $state<ScanSlice>({ baseline: null, started: false, done: false });
  projects    = $state<ProjectsSlice>({ projects: [], roles: {} });
  libraries   = $state<LibrariesSlice>({ libs: [], enabled: {}, extras: [] });
  instruments = $state<InstrumentsSlice>({ entries: [], enabled: {}, stack: null });
  inference   = $state<InferenceSlice>({ hardware: null, models: {}, apiKeys: {} });
  assignments = $state<AssignmentsSlice>({ mapping: {} });

  // ── Derived ──
  get firstPendingStage(): string    // first stage where completion !== 'done'
  get allDone(): boolean             // all stages done
  isStageComplete(id: string): boolean
  canAdvance(stageId: string): boolean  // validation gate

  // ── Lifecycle ──
  hydrate(data: WizardLoadData): void       // called by layout on mount
  async commitStage(id: string): Promise<boolean>  // POST + config key
  async commitAll(): Promise<void>          // final "Enter observatory"
}

export const wizardState = new WizardState();
```

Each slice is a plain interface. The `WizardState` class owns all mutation and commit logic. Slices have no methods.

### Slice Interfaces

```typescript
interface PreferencesSlice {
  displayName: string;
  contributeLearnings: boolean;
  reviewBeforeShare: boolean;
  shareSchedule: string;
  correctionAggressiveness: string;
  digestCadence: string;
  nudgeOnRegression: boolean;
  anonymizedTelemetry: boolean;
}

interface AssistantsSlice {
  families: DaemonAssistantFamily[];
  selected: Record<string, boolean>;  // id → toggled on (Record for Svelte 5 reactivity)
}

interface RootsSlice {
  roots: DaemonWatchRoot[];
  newPath: string;                 // input field state
}

interface ScanSlice {
  baseline: ScanBaseline | null;   // from daemon: previous counts
  started: boolean;
  done: boolean;
}

interface ScanBaseline {
  rootCount: number;
  repoCount: number;
  fileCount: number;
}

interface ProjectsSlice {
  projects: DaemonProject[];
  roles: Record<string, string>;   // folderId → role
}

interface LibrariesSlice {
  libs: DaemonLibEntry[];
  enabled: Record<string, boolean>;
  extras: { id: string; name: string; url: string }[];
}

interface InstrumentsSlice {
  entries: DaemonMcpEntry[];
  enabled: Record<string, boolean>;
  stack: DaemonDetectedStack | null;
}

interface InferenceSlice {
  hardware: DaemonHardwareInfo | null;
  models: Record<string, boolean>;
  apiKeys: Record<string, string>;
}

interface AssignmentsSlice {
  mapping: Record<string, string>;  // role → modelId
}
```

### Data Contracts (Daemon Response Types)

Live in `app/src/lib/setup/contracts.ts`. One interface per daemon endpoint response. These are the source of truth for what the daemon returns.

```typescript
interface DaemonWatchRoot {
  id: string;
  path: string;
  name: string;
  status: 'scanning' | 'watching' | 'paused';
  excluded: string[];
  repos_found: number;
  scanned: boolean;
  modified_at: string;
}

interface DaemonAssistantFamily {
  id: string;
  name: string;
  installed: boolean;
  config_path: string | null;
  version: string | null;
  install_path: string | null;
}

interface DaemonProject {
  id: string;
  name: string;
  description: string | null;
  client: string | null;
  goal: string | null;
  stack: { languages: string[]; frameworks: string[]; runtimes: string[]; services: string[] };
  icon: { kind: string; value: string };
  folders: { id: string; name: string; path: string; kind: string; role: string | null }[];
}

interface DaemonLibEntry {
  id: string;
  name: string;
  version: string;
  lang: string;
  usage: number;
  source: string;
  docs: 'indexed' | 'partial' | 'schema' | 'none';
}

interface DaemonMcpEntry {
  id: string;
  name: string;
  publisher: string;
  kind: string;
  summary: string;
  tools: number;
  verified: boolean;
  installed: boolean;
  recommended: boolean;
}

interface DaemonDetectedStack {
  languages: string[];
  frameworks: string[];
  runtimes: string[];
  services: string[];
}

interface DaemonHardwareInfo {
  ram_gb: number;
  gpu: string | null;
  recommended_tier: string;
}

// Bundle returned by layout load
interface WizardLoadData {
  completion: Record<string, 'pending' | 'done'>;
  preferences: Record<string, string>;
  assistantFamilies: DaemonAssistantFamily[];
  roots: DaemonWatchRoot[];
  projects: DaemonProject[];
  libraries: { total: number; libs: DaemonLibEntry[] };
  instruments: DaemonMcpEntry[];
  stack: DaemonDetectedStack | null;
  hardware: DaemonHardwareInfo | null;
}
```

### Load Pipeline

**`app/src/lib/setup/loaders.ts`** — single function, parallel fetch:

```typescript
export async function loadWizardData(port: number): Promise<WizardLoadData> {
  const api = senseiApi(port);
  const [config, families, roots, projects, libs] = await Promise.all([
    api.getConfig(),
    api.detectAssistantFamilies(),
    api.getScanRoots(),
    api.listProjects(),
    api.getLibs(),
    // instruments, hardware, stack — add when daemon endpoints exist
  ]);

  return {
    completion: extractCompletion(config),
    preferences: extractPreferences(config),
    assistantFamilies: families,
    roots,
    projects,
    libraries: libs,
    instruments: [],           // TBD: daemon endpoint
    stack: null,               // TBD: daemon endpoint
    hardware: null,            // TBD: daemon endpoint
  };
}

function extractCompletion(config: Record<string, string>): Record<string, 'pending' | 'done'> {
  const stages = ['welcome','preferences','assistants','roots','scan','projects','libraries','instruments','inference','assignments','done'];
  const result: Record<string, 'pending' | 'done'> = {};
  for (const s of stages) {
    result[s] = config[`setup.${s}`] === 'done' ? 'done' : 'pending';
  }
  return result;
}
```

**`app/src/routes/(config)/+layout.ts`** — SvelteKit load:

```typescript
import { loadWizardData } from '$lib/setup/loaders.js';
import { appState } from '$lib/appstate.svelte.js';

export async function load() {
  return await loadWizardData(appState.port);
}
```

**`app/src/routes/(config)/+layout.svelte`** — hydrates singleton, handles commit + navigation:

```
onMount: wizardState.hydrate(data)
Re-entry: if not first-time, goto(wizardState.firstPendingStage)

next():
  if canAdvance:
    success = await wizardState.commitStage(currentStageId)
    if success: goto(nextStagePath)
    else: show error
```

### Commit Pipeline

Each stage has a commit action. `commitStage(id)` dispatches to the right one:

| Stage | Commit action | Endpoint |
|-------|--------------|----------|
| welcome | Mark done | `PUT /api/config` |
| preferences | Save prefs to config | `PUT /api/config` (batch keys) |
| assistants | Configure selected ACPs | `POST /api/assistants/configure` |
| roots | Add new roots + exclusions | `POST /api/scan` per new root |
| scan | Mark done | `PUT /api/config` |
| projects | Confirm groupings + roles | `PUT /api/projects/:id` per project |
| libraries | Toggle lib indexing | `PUT /api/libs/configure` (TBD) |
| instruments | Toggle MCP selections | `POST /api/mcp/configure` (TBD) |
| inference | Save model + key selections | `PUT /api/config` (batch keys) |
| assignments | Save role→model mapping | `PUT /api/config` (batch keys) |

Every commit ends with: `api.setConfig({ 'setup.{stageId}': 'done' })`.

On failure, `commitStage()` returns `false` and the layout stays on the current page.

### Validation Gates

Checked by `canAdvance(stageId)`. Disables the Continue button when not met.

| Stage | Gate |
|-------|------|
| preferences | `displayName.trim()` not empty |
| roots | At least 1 root in list |
| scan | `scan.done === true` |
| All others | Always valid (optional selections) |

### Scan Stage — Incremental Awareness

Scan is the one stage that uses `EventManager` for live SSE. On re-entry (user added a new root), it shows:

- **Baseline** from `wizardState.scan.baseline`: previous counts loaded from daemon
- **Delta** from live SSE: new scan events only
- **UI**: `Roots: 3 (+1) · Repositories: 12 (+3) · Files: 2,400 (+180)`

The scan page creates `EventManager` on "Begin scan", subscribes to `/api/scan/events`, and destroys on completion or navigation away. `ScanProjectState` and `ScanActivityState` remain standalone classes, not part of `WizardState`.

### Re-Entry Flow

1. User navigates to `/setup/welcome` (or any setup route)
2. Layout load fetches all data from daemon
3. `wizardState.hydrate(data)` populates all slices
4. Layout checks `wizardState.firstPendingStage`
5. If current route is before first pending → redirect to first pending
6. Rail shows ✓ ticks on completed stages, all stages are navigable

User can click any completed stage to revisit. Clicking Continue on a revisited stage re-commits (idempotent).

## File Plan

### New files

| File | Purpose |
|------|---------|
| `src/lib/wizard-state.svelte.ts` | Singleton — slices, hydrate, commit, derived |
| `src/lib/setup/contracts.ts` | Daemon response interfaces |
| `src/lib/setup/loaders.ts` | `loadWizardData()` — parallel fetch |
| `src/routes/(config)/+layout.ts` | SvelteKit layout load |
| `src/lib/__tests__/wizard-state.test.ts` | Hydrate, commit, gates, firstPending |
| `src/lib/setup/__tests__/contracts.test.ts` | Validate mock data matches contract shapes |
| `src/lib/setup/__tests__/loaders.test.ts` | Load function with mocked API |

### Modified files

| File | Change |
|------|--------|
| `src/routes/(config)/+layout.svelte` | Hydrate singleton from `data`, wire `commitStage` into `next()`, disable Continue when gate fails |
| `src/routes/(config)/stages.ts` | Rename Folders → Roots (id, path, icon update) |
| `src/routes/(config)/setup/assistants/+page.svelte` | Read from `wizardState.assistants` instead of `data` prop |
| `src/routes/(config)/setup/scan/+page.svelte` | Read baseline from `wizardState.scan`, keep EventManager for live SSE |
| `src/lib/setup/types.ts` | Remove `WIZ_STAGES` and `WizardState` (moved/replaced). Keep entity types that aren't covered by contracts. |
| `src/lib/setup/mock.ts` | Update to match new contract types |

### Renamed

| From | To |
|------|-----|
| `src/routes/(config)/setup/folders/` | `src/routes/(config)/setup/roots/` |

### Removed

| File | Why |
|------|-----|
| `src/routes/(config)/setup/assistants/+page.ts` | Load moves to layout |
| `src/routes/(config)/setup/folders/+page.ts` | Load moves to layout |

### Unchanged

| File | Why |
|------|-----|
| `src/lib/stage.svelte.ts` | Still used by scan |
| `src/lib/events.ts` | Still used by scan |
| `src/lib/scan-state.svelte.ts` | Scan-specific, not part of wizard state |
| `src/lib/api.ts` | Shape stays, may need new endpoint methods |
| `src/lib/appstate.svelte.ts` | Unchanged |

## Daemon API Gaps

Endpoints that need to be added or updated on the daemon side:

| Endpoint | Status | Needed for |
|----------|--------|------------|
| `POST /api/scan/roots` (add root with exclusions) | Missing — currently `POST /api/scan` implicitly adds | Roots stage commit |
| `PUT /api/scan/roots/:id` (update exclusions) | Missing | Roots stage — edit exclusions |
| `DELETE /api/scan/roots/:id` (remove root) | Exists in DB (`remove_watch_root`) but no HTTP route | Roots stage |
| `GET /api/mcp/available` (list MCP registry) | Missing | Instruments stage |
| `POST /api/mcp/configure` (toggle MCPs) | Missing | Instruments stage commit |
| `PUT /api/libs/configure` (toggle lib indexing) | Missing | Libraries stage commit |
| `GET /api/system/hardware` (RAM, GPU) | Missing | Inference stage |
| `GET /api/inference/models` (available models) | Missing | Inference stage |
| SSE scan events — incremental mode | Needs update — should include baseline counts | Scan stage re-entry |

These can be built incrementally as each stage is implemented.

## Testing Strategy

1. **Contract tests**: Validate that mock data satisfies the `Daemon*` interfaces. Catches drift between app assumptions and daemon responses.
2. **Hydration tests**: Given `WizardLoadData`, assert each slice is populated correctly.
3. **Commit tests**: Mock the API, call `commitStage()`, assert correct endpoint called with correct payload, assert config key updated.
4. **Gate tests**: Assert `canAdvance()` returns false when validation fails, true when it passes.
5. **Re-entry tests**: Hydrate with partial completion, assert `firstPendingStage` returns correct stage.
6. **Scan baseline tests**: Hydrate with existing scan data, assert baseline counts are correct.
