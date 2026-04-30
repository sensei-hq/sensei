# Wizard State Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish a canonical data flow for the 11-stage setup wizard: layout load from daemon, shared singleton state, per-stage commit on advance, re-entry support.

**Architecture:** Thin singleton `WizardState` hydrated by a parallel layout load. Each stage reads/writes slices on the singleton. Advancing commits the stage's data to the daemon via API, then marks the config key as done. Scan stage keeps its standalone SSE + ReactiveStageContext classes.

**Tech Stack:** SvelteKit, Svelte 5 (`$state`/`$derived`), TypeScript, vitest, daemon REST API

**Spec:** `docs/superpowers/specs/2026-04-30-wizard-state-architecture-design.md`

---

## File Structure

| File | Responsibility |
|------|---------------|
| `src/lib/setup/contracts.ts` | Daemon response interfaces — source of truth for API shapes |
| `src/lib/setup/mock-contracts.ts` | Factory functions returning valid mock data per contract |
| `src/lib/setup/loaders.ts` | `loadWizardData()` — parallel fetch from daemon, returns `WizardLoadData` |
| `src/lib/wizard-state.svelte.ts` | Singleton — slices, hydrate, commitStage, canAdvance, firstPendingStage |
| `src/routes/(config)/+layout.ts` | SvelteKit load — calls `loadWizardData()` |
| `src/routes/(config)/+layout.svelte` | Hydrate singleton, wire commitStage into next(), re-entry redirect |
| `src/routes/(config)/stages.ts` | Stage definitions — already updated (meaning kanji, 11 stages) |
| `src/routes/(config)/setup/roots/+page.svelte` | Renamed from folders/ — reads `wizardState.roots` |
| `src/routes/(config)/setup/assistants/+page.svelte` | Refactored — reads `wizardState.assistants` |
| `src/routes/(config)/setup/preferences/+page.svelte` | New — reads `wizardState.preferences` |
| `src/routes/(config)/setup/scan/+page.svelte` | Updated — reads baseline from `wizardState.scan` |
| `src/routes/(config)/setup/projects/+page.svelte` | New — reads `wizardState.projects` |
| `src/routes/(config)/setup/libraries/+page.svelte` | New — reads `wizardState.libraries` |
| `src/routes/(config)/setup/instruments/+page.svelte` | New — reads `wizardState.instruments` |
| Tests: `src/lib/setup/contracts.spec.svelte.ts` | Contract shape validation |
| Tests: `src/lib/wizard-state.spec.svelte.ts` | Hydrate, commit, gates, firstPending |
| Tests: `src/lib/setup/loaders.spec.svelte.ts` | Loader with mocked API |

---

## Task 1: Contracts + Mock Data

**Files:**
- Create: `src/lib/setup/contracts.ts`
- Create: `src/lib/setup/mock-contracts.ts`
- Create: `src/lib/setup/contracts.spec.svelte.ts`
- Modify: `src/lib/setup/types.ts` — remove `WIZ_STAGES` and old `WizardState`

- [ ] **Step 1: Write the contract interfaces**

Create `src/lib/setup/contracts.ts` with all daemon response types and the `WizardLoadData` bundle. Every interface is exported. These are the source of truth.

```typescript
// src/lib/setup/contracts.ts

/** Daemon response: a watch root directory. */
export interface DaemonWatchRoot {
  id: string;
  path: string;
  name: string;
  status: 'scanning' | 'watching' | 'paused';
  excluded: string[];
  repos_found: number;
  scanned: boolean;
  modified_at: string;
}

/** Daemon response: an AI coding assistant family. */
export interface DaemonAssistantFamily {
  id: string;
  name: string;
  installed: boolean;
  selected: boolean;
  config_path: string | null;
  version: string | null;
  install_path: string | null;
}

/** Daemon response: a project with its folders. */
export interface DaemonProject {
  id: string;
  name: string;
  description: string | null;
  client: string | null;
  goal: string | null;
  stack: { languages: string[]; frameworks: string[]; runtimes: string[]; services: string[] };
  icon: { kind: string; value: string };
  folders: DaemonProjectFolder[];
}

export interface DaemonProjectFolder {
  id: string;
  name: string;
  path: string;
  kind: string;
  role: string | null;
}

/** Daemon response: a detected library. */
export interface DaemonLibEntry {
  id: string;
  name: string;
  version: string;
  lang: string;
  usage: number;
  source: string;
  docs: 'indexed' | 'partial' | 'schema' | 'none';
  enabled: boolean;
}

/** Daemon response: an MCP server entry. */
export interface DaemonMcpEntry {
  id: string;
  name: string;
  publisher: string;
  kind: string;
  summary: string;
  tools: number;
  verified: boolean;
  installed: boolean;
  recommended: boolean;
  selected: boolean;
  project_count: number;
}

/** Scan baseline — derived from loaded data. */
export interface ScanBaseline {
  rootCount: number;
  repoCount: number;
  fileCount: number;
  scannedRootIds: string[];
}

/** Preferences stored as config key-value pairs. */
export interface PreferencesData {
  displayName: string;
  contributeLearnings: boolean;
  reviewBeforeShare: boolean;
  shareSchedule: string;
  correctionAggressiveness: string;
  digestCadence: string;
  nudgeOnRegression: boolean;
  anonymizedTelemetry: boolean;
}

/** Bundle returned by layout load. */
export interface WizardLoadData {
  completion: Record<string, 'pending' | 'done'>;
  preferences: PreferencesData;
  assistantFamilies: DaemonAssistantFamily[];
  roots: DaemonWatchRoot[];
  projects: DaemonProject[];
  libraries: { total: number; libs: DaemonLibEntry[] };
  mcps: DaemonMcpEntry[];
}
```

- [ ] **Step 2: Write mock factory functions**

Create `src/lib/setup/mock-contracts.ts`. Each function returns one valid instance of a contract type. Used by tests.

```typescript
// src/lib/setup/mock-contracts.ts
import type {
  DaemonWatchRoot, DaemonAssistantFamily, DaemonProject,
  DaemonLibEntry, DaemonMcpEntry, PreferencesData, WizardLoadData,
} from './contracts.js';

export function mockWatchRoot(overrides: Partial<DaemonWatchRoot> = {}): DaemonWatchRoot {
  return {
    id: 'root-1', path: '/Users/test/code', name: 'Code',
    status: 'watching', excluded: ['node_modules', '.git'],
    repos_found: 3, scanned: true, modified_at: '2026-04-30T00:00:00Z',
    ...overrides,
  };
}

export function mockAssistant(overrides: Partial<DaemonAssistantFamily> = {}): DaemonAssistantFamily {
  return {
    id: 'claude-code', name: 'Claude Code', installed: true, selected: true,
    config_path: '~/.claude/config.json', version: '1.0.0', install_path: '/usr/local/bin/claude',
    ...overrides,
  };
}

export function mockProject(overrides: Partial<DaemonProject> = {}): DaemonProject {
  return {
    id: 'proj-1', name: 'Test Project', description: null, client: null, goal: null,
    stack: { languages: ['TypeScript'], frameworks: ['SvelteKit'], runtimes: ['Node 20'], services: [] },
    icon: { kind: 'kanji', value: '工' },
    folders: [
      { id: 'f-1', name: 'app', path: '/Users/test/code/app', kind: 'git', role: 'frontend' },
    ],
    ...overrides,
  };
}

export function mockLibEntry(overrides: Partial<DaemonLibEntry> = {}): DaemonLibEntry {
  return {
    id: 'svelte', name: 'svelte', version: '5.0.0', lang: 'TypeScript',
    usage: 42, source: 'package.json', docs: 'indexed', enabled: true,
    ...overrides,
  };
}

export function mockMcpEntry(overrides: Partial<DaemonMcpEntry> = {}): DaemonMcpEntry {
  return {
    id: 'postgres-mcp', name: 'PostgreSQL MCP', publisher: 'supabase', kind: 'data',
    summary: 'Query schema, introspect tables.', tools: 14, verified: true,
    installed: false, recommended: true, selected: true, project_count: 2,
    ...overrides,
  };
}

export function mockPreferences(overrides: Partial<PreferencesData> = {}): PreferencesData {
  return {
    displayName: 'Jerry', contributeLearnings: true, reviewBeforeShare: true,
    shareSchedule: 'weekly-saturday', correctionAggressiveness: 'balanced',
    digestCadence: 'daily', nudgeOnRegression: true, anonymizedTelemetry: false,
    ...overrides,
  };
}

export function mockWizardLoadData(overrides: Partial<WizardLoadData> = {}): WizardLoadData {
  return {
    completion: {
      welcome: 'pending', preferences: 'pending', assistants: 'pending',
      roots: 'pending', scan: 'pending', projects: 'pending',
      libraries: 'pending', instruments: 'pending',
      inference: 'pending', assignments: 'pending', done: 'pending',
    },
    preferences: mockPreferences(),
    assistantFamilies: [mockAssistant(), mockAssistant({ id: 'cursor', name: 'Cursor', installed: false, selected: false })],
    roots: [mockWatchRoot()],
    projects: [mockProject()],
    libraries: { total: 1, libs: [mockLibEntry()] },
    mcps: [mockMcpEntry()],
    ...overrides,
  };
}
```

- [ ] **Step 3: Write contract shape tests**

Create `src/lib/setup/contracts.spec.svelte.ts`. Validates that mock data satisfies the contract interfaces.

```typescript
// src/lib/setup/contracts.spec.svelte.ts
import { describe, it, expect } from 'vitest';
import {
  mockWatchRoot, mockAssistant, mockProject, mockLibEntry,
  mockMcpEntry, mockPreferences, mockWizardLoadData,
} from './mock-contracts.js';

describe('contract mock factories', () => {
  it('mockWatchRoot has required fields', () => {
    const root = mockWatchRoot();
    expect(root.id).toBeTypeOf('string');
    expect(root.path).toBeTypeOf('string');
    expect(['scanning', 'watching', 'paused']).toContain(root.status);
    expect(Array.isArray(root.excluded)).toBe(true);
    expect(root.repos_found).toBeTypeOf('number');
  });

  it('mockAssistant has required fields', () => {
    const a = mockAssistant();
    expect(a.id).toBeTypeOf('string');
    expect(a.installed).toBeTypeOf('boolean');
    expect(a.selected).toBeTypeOf('boolean');
  });

  it('mockProject has folders array', () => {
    const p = mockProject();
    expect(Array.isArray(p.folders)).toBe(true);
    expect(p.folders[0].kind).toBeTypeOf('string');
  });

  it('mockLibEntry has docs status', () => {
    const l = mockLibEntry();
    expect(['indexed', 'partial', 'schema', 'none']).toContain(l.docs);
    expect(l.enabled).toBeTypeOf('boolean');
  });

  it('mockMcpEntry has project_count', () => {
    const m = mockMcpEntry();
    expect(m.project_count).toBeTypeOf('number');
    expect(m.selected).toBeTypeOf('boolean');
  });

  it('mockWizardLoadData has all slices', () => {
    const data = mockWizardLoadData();
    expect(data.completion).toBeDefined();
    expect(data.preferences.displayName).toBeTypeOf('string');
    expect(Array.isArray(data.assistantFamilies)).toBe(true);
    expect(Array.isArray(data.roots)).toBe(true);
    expect(Array.isArray(data.projects)).toBe(true);
    expect(Array.isArray(data.libraries.libs)).toBe(true);
    expect(Array.isArray(data.mcps)).toBe(true);
  });

  it('mockWatchRoot accepts overrides', () => {
    const root = mockWatchRoot({ status: 'paused', repos_found: 0 });
    expect(root.status).toBe('paused');
    expect(root.repos_found).toBe(0);
  });

  it('mockWizardLoadData accepts partial completion', () => {
    const data = mockWizardLoadData({
      completion: {
        welcome: 'done', preferences: 'done', assistants: 'done',
        roots: 'done', scan: 'done', projects: 'pending',
        libraries: 'pending', instruments: 'pending',
        inference: 'pending', assignments: 'pending', done: 'pending',
      },
    });
    expect(data.completion.welcome).toBe('done');
    expect(data.completion.projects).toBe('pending');
  });
});
```

- [ ] **Step 4: Run tests**

Run: `cd /Users/Jerry/Developer/sensei/app && bunx vitest run src/lib/setup/contracts.spec.svelte.ts`
Expected: all tests PASS

- [ ] **Step 5: Clean up old types**

Modify `src/lib/setup/types.ts` — remove `WIZ_STAGES` and old `WizardState` interface (both moved/replaced). Keep entity types that aren't covered by contracts (e.g. `RoleOption`, `ROLES`).

- [ ] **Step 6: Run full test suite + type check**

Run: `bunx vitest run && npx svelte-check --tsconfig ./tsconfig.json`
Expected: 0 errors, all tests pass

- [ ] **Step 7: Commit**

```bash
git add src/lib/setup/contracts.ts src/lib/setup/mock-contracts.ts src/lib/setup/contracts.spec.svelte.ts src/lib/setup/types.ts
git commit -m "feat(setup): add wizard data contracts and mock factories"
```

---

## Task 2: WizardState Singleton

**Files:**
- Create: `src/lib/wizard-state.svelte.ts`
- Create: `src/lib/wizard-state.spec.svelte.ts`

- [ ] **Step 1: Write hydration tests**

```typescript
// src/lib/wizard-state.spec.svelte.ts
import { describe, it, expect, beforeEach } from 'vitest';
import { WizardState } from './wizard-state.svelte.js';
import { mockWizardLoadData, mockWatchRoot, mockAssistant, mockPreferences } from './setup/mock-contracts.js';

describe('WizardState', () => {
  let ws: WizardState;

  beforeEach(() => {
    ws = new WizardState();
  });

  describe('hydrate', () => {
    it('populates completion from load data', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.completion.welcome).toBe('pending');
    });

    it('populates preferences slice', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.preferences.displayName).toBe('Jerry');
    });

    it('populates assistants with selected defaulting to installed', () => {
      const data = mockWizardLoadData();
      ws.hydrate(data);
      expect(ws.assistants.assistants).toHaveLength(2);
      expect(ws.assistants.assistants[0].selected).toBe(true);  // installed
      expect(ws.assistants.assistants[1].selected).toBe(false); // not installed
    });

    it('populates roots slice', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.roots.roots).toHaveLength(1);
      expect(ws.roots.roots[0].path).toBe('/Users/test/code');
    });

    it('populates scan baseline from roots', () => {
      const data = mockWizardLoadData({
        roots: [
          mockWatchRoot({ id: 'r1', repos_found: 3, scanned: true }),
          mockWatchRoot({ id: 'r2', repos_found: 2, scanned: true }),
        ],
      });
      ws.hydrate(data);
      expect(ws.scan.baseline).toEqual({
        rootCount: 2,
        repoCount: 5,
        fileCount: 0,
        scannedRootIds: ['r1', 'r2'],
      });
    });

    it('populates projects slice', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.projects.projects).toHaveLength(1);
    });

    it('populates libraries slice', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.libraries.libs).toHaveLength(1);
    });

    it('populates instruments slice', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.instruments.mcps).toHaveLength(1);
    });
  });

  describe('firstPendingStage', () => {
    it('returns welcome when nothing done', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.firstPendingStage).toBe('welcome');
    });

    it('returns first pending after some done', () => {
      ws.hydrate(mockWizardLoadData({
        completion: {
          welcome: 'done', preferences: 'done', assistants: 'done',
          roots: 'pending', scan: 'pending', projects: 'pending',
          libraries: 'pending', instruments: 'pending',
          inference: 'pending', assignments: 'pending', done: 'pending',
        },
      }));
      expect(ws.firstPendingStage).toBe('roots');
    });

    it('returns done when all stages complete', () => {
      const allDone: Record<string, 'done'> = {
        welcome: 'done', preferences: 'done', assistants: 'done',
        roots: 'done', scan: 'done', projects: 'done',
        libraries: 'done', instruments: 'done',
        inference: 'done', assignments: 'done', done: 'done',
      };
      ws.hydrate(mockWizardLoadData({ completion: allDone }));
      expect(ws.allDone).toBe(true);
    });
  });

  describe('canAdvance', () => {
    beforeEach(() => ws.hydrate(mockWizardLoadData()));

    it('returns true for welcome (no gate)', () => {
      expect(ws.canAdvance('welcome')).toBe(true);
    });

    it('returns false for preferences when displayName empty', () => {
      ws.preferences.displayName = '';
      expect(ws.canAdvance('preferences')).toBe(false);
    });

    it('returns true for preferences when displayName set', () => {
      ws.preferences.displayName = 'Jerry';
      expect(ws.canAdvance('preferences')).toBe(true);
    });

    it('returns false for roots when list empty', () => {
      ws.roots.roots = [];
      expect(ws.canAdvance('roots')).toBe(false);
    });

    it('returns true for roots when list has entries', () => {
      expect(ws.canAdvance('roots')).toBe(true);
    });

    it('returns false for scan when not done', () => {
      ws.scan.done = false;
      expect(ws.canAdvance('scan')).toBe(false);
    });

    it('returns true for scan when done', () => {
      ws.scan.done = true;
      expect(ws.canAdvance('scan')).toBe(true);
    });

    it('returns true for stages with no gate', () => {
      expect(ws.canAdvance('assistants')).toBe(true);
      expect(ws.canAdvance('projects')).toBe(true);
      expect(ws.canAdvance('libraries')).toBe(true);
      expect(ws.canAdvance('instruments')).toBe(true);
    });
  });

  describe('isStageComplete', () => {
    it('returns false when pending', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.isStageComplete('welcome')).toBe(false);
    });

    it('returns true when done', () => {
      ws.hydrate(mockWizardLoadData({
        completion: {
          welcome: 'done', preferences: 'pending', assistants: 'pending',
          roots: 'pending', scan: 'pending', projects: 'pending',
          libraries: 'pending', instruments: 'pending',
          inference: 'pending', assignments: 'pending', done: 'pending',
        },
      }));
      expect(ws.isStageComplete('welcome')).toBe(true);
    });
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `bunx vitest run src/lib/wizard-state.spec.svelte.ts`
Expected: FAIL — module not found

- [ ] **Step 3: Implement WizardState**

Create `src/lib/wizard-state.svelte.ts`:

```typescript
// src/lib/wizard-state.svelte.ts
import { senseiApi } from './api.js';
import { appState } from './appstate.svelte.js';
import type {
  DaemonAssistantFamily, DaemonWatchRoot, DaemonProject,
  DaemonLibEntry, DaemonMcpEntry, PreferencesData,
  ScanBaseline, WizardLoadData,
} from './setup/contracts.js';

// ── Slice interfaces ────────────────────────────────────────

export interface AssistantsSlice {
  assistants: DaemonAssistantFamily[];
}

export interface RootsSlice {
  roots: DaemonWatchRoot[];
  newPath: string;
}

export interface ScanSlice {
  baseline: ScanBaseline | null;
  started: boolean;
  done: boolean;
}

export interface ProjectsSlice {
  projects: DaemonProject[];
}

export interface LibrariesSlice {
  libs: DaemonLibEntry[];
}

export interface InstrumentsSlice {
  mcps: DaemonMcpEntry[];
}

// ── Stage ID order (for firstPendingStage) ──────────────────

const STAGE_ORDER = [
  'welcome', 'preferences', 'assistants', 'roots', 'scan',
  'projects', 'libraries', 'instruments',
  'inference', 'assignments', 'done',
] as const;

// ── WizardState ─────────────────────────────────────────────

export class WizardState {
  completion = $state<Record<string, 'pending' | 'done'>>({});

  preferences = $state<PreferencesData>({
    displayName: '', contributeLearnings: true, reviewBeforeShare: true,
    shareSchedule: 'weekly-saturday', correctionAggressiveness: 'balanced',
    digestCadence: 'daily', nudgeOnRegression: true, anonymizedTelemetry: false,
  });
  assistants  = $state<AssistantsSlice>({ assistants: [] });
  roots       = $state<RootsSlice>({ roots: [], newPath: '' });
  scan        = $state<ScanSlice>({ baseline: null, started: false, done: false });
  projects    = $state<ProjectsSlice>({ projects: [] });
  libraries   = $state<LibrariesSlice>({ libs: [] });
  instruments = $state<InstrumentsSlice>({ mcps: [] });

  // ── Derived ──

  get firstPendingStage(): string {
    for (const id of STAGE_ORDER) {
      if (this.completion[id] !== 'done') return id;
    }
    return 'done';
  }

  get allDone(): boolean {
    return STAGE_ORDER.every(id => this.completion[id] === 'done');
  }

  isStageComplete(id: string): boolean {
    return this.completion[id] === 'done';
  }

  canAdvance(stageId: string): boolean {
    switch (stageId) {
      case 'preferences': return this.preferences.displayName.trim().length > 0;
      case 'roots':       return this.roots.roots.length > 0;
      case 'scan':        return this.scan.done;
      default:            return true;
    }
  }

  // ── Lifecycle ──

  hydrate(data: WizardLoadData): void {
    this.completion = { ...data.completion };
    this.preferences = { ...data.preferences };

    this.assistants = {
      assistants: data.assistantFamilies.map(a => ({ ...a, selected: a.selected ?? a.installed })),
    };

    this.roots = { roots: [...data.roots], newPath: '' };

    const scannedRoots = data.roots.filter(r => r.scanned);
    this.scan = {
      baseline: {
        rootCount: scannedRoots.length,
        repoCount: data.roots.reduce((sum, r) => sum + r.repos_found, 0),
        fileCount: 0,
        scannedRootIds: scannedRoots.map(r => r.id),
      },
      started: false,
      done: false,
    };

    this.projects = { projects: [...data.projects] };
    this.libraries = { libs: [...data.libraries.libs] };
    this.instruments = { mcps: [...data.mcps] };
  }

  async commitStage(stageId: string): Promise<boolean> {
    const api = senseiApi(appState.port);
    try {
      switch (stageId) {
        case 'welcome':
          break;

        case 'preferences':
          await api.setConfig(this.prefsToConfig());
          break;

        case 'assistants': {
          const selected = this.assistants.assistants.filter(a => a.selected).map(a => a.id);
          await api.configureAssistants(selected);
          break;
        }

        case 'roots':
          for (const root of this.roots.roots) {
            if (!root.scanned) {
              await api.scanFolder(root.path);
            }
          }
          break;

        case 'scan':
          break;

        case 'projects':
          for (const project of this.projects.projects) {
            await api.updateProject(project.id, project);
          }
          break;

        case 'libraries':
          // TBD: api.configureLibs(this.libraries.libs)
          break;

        case 'instruments':
          // TBD: api.configureMcps(this.instruments.mcps)
          break;

        case 'inference':
        case 'assignments':
          // DEFERRED
          break;

        case 'done':
          await appState.setSetupComplete();
          break;
      }

      await api.setConfig({ [`setup.${stageId}`]: 'done' });
      this.completion[stageId] = 'done';
      return true;
    } catch {
      return false;
    }
  }

  private prefsToConfig(): Record<string, string> {
    return {
      'pref.displayName': this.preferences.displayName,
      'pref.contributeLearnings': String(this.preferences.contributeLearnings),
      'pref.reviewBeforeShare': String(this.preferences.reviewBeforeShare),
      'pref.shareSchedule': this.preferences.shareSchedule,
      'pref.correctionAggressiveness': this.preferences.correctionAggressiveness,
      'pref.digestCadence': this.preferences.digestCadence,
      'pref.nudgeOnRegression': String(this.preferences.nudgeOnRegression),
      'pref.anonymizedTelemetry': String(this.preferences.anonymizedTelemetry),
    };
  }
}

export const wizardState = new WizardState();
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `bunx vitest run src/lib/wizard-state.spec.svelte.ts`
Expected: all tests PASS

- [ ] **Step 5: Run full test suite + type check**

Run: `bunx vitest run && npx svelte-check --tsconfig ./tsconfig.json`
Expected: 0 errors, all tests pass

- [ ] **Step 6: Commit**

```bash
git add src/lib/wizard-state.svelte.ts src/lib/wizard-state.spec.svelte.ts
git commit -m "feat(setup): add WizardState singleton with hydrate, commit, gates"
```

---

## Task 3: Loaders + Layout Integration

**Files:**
- Create: `src/lib/setup/loaders.ts`
- Create: `src/lib/setup/loaders.spec.svelte.ts`
- Create: `src/routes/(config)/+layout.ts`
- Modify: `src/routes/(config)/+layout.svelte`
- Modify: `src/routes/(config)/stages.ts` — rename Folders → Roots

- [ ] **Step 1: Write loader tests**

```typescript
// src/lib/setup/loaders.spec.svelte.ts
import { describe, it, expect, vi } from 'vitest';
import { extractCompletion, extractPreferences } from './loaders.js';

describe('extractCompletion', () => {
  it('returns pending for missing keys', () => {
    const result = extractCompletion({});
    expect(result.welcome).toBe('pending');
    expect(result.roots).toBe('pending');
  });

  it('returns done for set keys', () => {
    const result = extractCompletion({ 'setup.welcome': 'done', 'setup.roots': 'done' });
    expect(result.welcome).toBe('done');
    expect(result.roots).toBe('done');
    expect(result.scan).toBe('pending');
  });
});

describe('extractPreferences', () => {
  it('returns defaults when no config keys', () => {
    const result = extractPreferences({});
    expect(result.displayName).toBe('');
    expect(result.contributeLearnings).toBe(true);
  });

  it('parses config keys into PreferencesData', () => {
    const result = extractPreferences({
      'pref.displayName': 'Jerry',
      'pref.contributeLearnings': 'false',
      'pref.anonymizedTelemetry': 'true',
    });
    expect(result.displayName).toBe('Jerry');
    expect(result.contributeLearnings).toBe(false);
    expect(result.anonymizedTelemetry).toBe(true);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `bunx vitest run src/lib/setup/loaders.spec.svelte.ts`
Expected: FAIL — module not found

- [ ] **Step 3: Implement loaders**

Create `src/lib/setup/loaders.ts`:

```typescript
// src/lib/setup/loaders.ts
import { senseiApi } from '$lib/api.js';
import type { PreferencesData, WizardLoadData } from './contracts.js';

const STAGES = [
  'welcome', 'preferences', 'assistants', 'roots', 'scan',
  'projects', 'libraries', 'instruments', 'inference', 'assignments', 'done',
];

const PREF_DEFAULTS: PreferencesData = {
  displayName: '', contributeLearnings: true, reviewBeforeShare: true,
  shareSchedule: 'weekly-saturday', correctionAggressiveness: 'balanced',
  digestCadence: 'daily', nudgeOnRegression: true, anonymizedTelemetry: false,
};

export function extractCompletion(config: Record<string, string>): Record<string, 'pending' | 'done'> {
  const result: Record<string, 'pending' | 'done'> = {};
  for (const s of STAGES) {
    result[s] = config[`setup.${s}`] === 'done' ? 'done' : 'pending';
  }
  return result;
}

export function extractPreferences(config: Record<string, string>): PreferencesData {
  const str = (key: string, fallback: string) => config[`pref.${key}`] ?? fallback;
  const bool = (key: string, fallback: boolean) => {
    const v = config[`pref.${key}`];
    return v === undefined ? fallback : v === 'true';
  };

  return {
    displayName:             str('displayName', PREF_DEFAULTS.displayName),
    contributeLearnings:     bool('contributeLearnings', PREF_DEFAULTS.contributeLearnings),
    reviewBeforeShare:       bool('reviewBeforeShare', PREF_DEFAULTS.reviewBeforeShare),
    shareSchedule:           str('shareSchedule', PREF_DEFAULTS.shareSchedule),
    correctionAggressiveness:str('correctionAggressiveness', PREF_DEFAULTS.correctionAggressiveness),
    digestCadence:           str('digestCadence', PREF_DEFAULTS.digestCadence),
    nudgeOnRegression:       bool('nudgeOnRegression', PREF_DEFAULTS.nudgeOnRegression),
    anonymizedTelemetry:     bool('anonymizedTelemetry', PREF_DEFAULTS.anonymizedTelemetry),
  };
}

export async function loadWizardData(port: number): Promise<WizardLoadData> {
  const api = senseiApi(port);
  const [config, families, roots, projects, libs] = await Promise.all([
    api.getConfig(),
    api.detectAssistantFamilies(),
    api.getScanRoots(),
    api.listProjects(),
    api.getLibs(),
  ]);

  return {
    completion: extractCompletion(config),
    preferences: extractPreferences(config),
    assistantFamilies: families as any[],
    roots: roots as any[],
    projects: projects as any[],
    libraries: libs as any,
    mcps: [],
  };
}
```

- [ ] **Step 4: Run loader tests**

Run: `bunx vitest run src/lib/setup/loaders.spec.svelte.ts`
Expected: PASS

- [ ] **Step 5: Rename Folders → Roots in stages.ts**

Update `src/routes/(config)/stages.ts`: change the `folders` stage to `roots`:

```typescript
// Change this line:
{ id: 'folders', path: '/setup/folders', icon: '庵', title: 'Folders', sub: 'where does your work live', watermark: true },
// To:
{ id: 'roots', path: '/setup/roots', icon: '庵', title: 'Roots', sub: 'where does your work live', watermark: true },
```

- [ ] **Step 6: Rename folders/ directory to roots/**

```bash
mv src/routes/(config)/setup/folders src/routes/(config)/setup/roots
```

- [ ] **Step 7: Create layout load function**

Create `src/routes/(config)/+layout.ts`:

```typescript
import { loadWizardData } from '$lib/setup/loaders.js';
import { appState } from '$lib/appstate.svelte.js';

export async function load() {
  return await loadWizardData(appState.port);
}
```

- [ ] **Step 8: Update layout.svelte — hydrate + commit + gates**

Update `src/routes/(config)/+layout.svelte` to:
- Import `wizardState` and hydrate from `data` on mount
- Wire `commitStage` into `next()` — await commit before navigating
- Bind Continue button disabled state to `canAdvance`
- Add `committing` state to show loading during commit
- Handle re-entry: redirect to `firstPendingStage` if current page is behind it

Key changes in the script block:

```typescript
import { wizardState } from '$lib/wizard-state.svelte.js';
import { onMount } from 'svelte';

let { children, data } = $props();

onMount(() => {
  wizardState.hydrate(data);
});

const canAdvance = $derived(wizardState.canAdvance(stage?.id ?? ''));
let committing = $state(false);

async function next() {
  if (isLast) {
    committing = true;
    await wizardState.commitStage('done');
    committing = false;
    goto('/observatory');
    return;
  }
  if (!canAdvance) return;
  committing = true;
  const ok = await wizardState.commitStage(stage.id);
  committing = false;
  if (ok) {
    const path = nextStagePath(page.url.pathname);
    if (path) goto(path);
  }
}
```

Update the Continue button to respect `canAdvance` and `committing`:

```svelte
<button class="btn-primary" onclick={next} disabled={!canAdvance || committing}>
  {committing ? 'Saving...' : isLast ? 'Enter observatory →' : 'Continue →'}
</button>
```

Update rail items to use `wizardState.isStageComplete(s.id)` for ✓ ticks and allow clicking any completed or current stage.

- [ ] **Step 9: Remove old page-level load functions**

Delete `src/routes/(config)/setup/assistants/+page.ts` and `src/routes/(config)/setup/roots/+page.ts` (was folders/+page.ts).

- [ ] **Step 10: Run full test suite + type check**

Run: `bunx vitest run && npx svelte-check --tsconfig ./tsconfig.json`
Expected: 0 errors, all tests pass

- [ ] **Step 11: Start dev server and verify**

Run: `bun run dev`
Navigate to `http://localhost:5173/setup/welcome`
Verify: rail renders, Continue works, stages advance, ✓ ticks appear on completed stages

- [ ] **Step 12: Commit**

```bash
git add -A
git commit -m "feat(setup): add layout load pipeline, wizard state hydration, commit on advance"
```

---

## Next Steps

Per-stage implementation is tracked in `docs/backlog.md`. Each stage gets its own detailed plan when picked up. See `docs/superpowers/specs/2026-04-30-wizard-state-architecture-design.md` for the full spec.

Stage order: Welcome → Preferences → Assistants → Roots → Scan → Projects → Libraries → Instruments → Done. Inference and Assignments are deferred pending gateway design.

---

## Notes for the implementing engineer

- **Test pattern**: vitest with `*.spec.svelte.ts` files alongside the module they test (the `.svelte.ts` extension enables Svelte 5 reactivity in tests). Import from `vitest` (`describe`, `it`, `expect`, `vi`).
- **Svelte 5 reactivity**: Use `$state` for mutable state, `$derived` for computed. `$effect` for side effects. No `writable()` stores.
- **API client**: `senseiApi(appState.port)` returns a typed client. All methods have fallback values — they never throw.
- **Existing scan infrastructure**: `EventManager` (events.ts), `ScanProjectState` / `ScanActivityState` (scan-state.svelte.ts), `ReactiveStageContext` (stage.svelte.ts) — these stay as-is. Only the scan page uses them.
- **Zero errors policy**: Every commit must pass `bunx vitest run && npx svelte-check --tsconfig ./tsconfig.json` with 0 errors.
- **Branch**: All work on `develop` branch.
