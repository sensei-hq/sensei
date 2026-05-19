/**
 * WizardState — singleton managing all setup wizard data.
 *
 * Hydrated from daemon via layout load. Stages read/write slices.
 * commitStage() persists to daemon on advance.
 *
 * Stage metadata + live status + active flag all live on `stages` (one
 * WizardStage[] array). The rail and the page header consume the same
 * objects; status persists via commitStage; active flips on route change.
 */

import { senseiApi } from './api.js';
import { appState } from './appstate.svelte.js';
import { hasTauri } from './bootstrap.js';
import { STAGES, type WizardStage } from '../routes/(config)/stages.js';
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

// ── Commit handlers ─────────────────────────────────────────

type CommitFn = (ws: WizardState, api: ReturnType<typeof senseiApi>) => Promise<void>;

const COMMIT_HANDLERS: Record<string, CommitFn> = {
  welcome:     async () => {},
  preferences: async (ws, api) => {
    await api.setConfig({
      'setup.preferences': JSON.stringify(ws.preferences),
      'user_name': ws.preferences.displayName,
    });
  },
  assistants:  async (ws, api) => {
    const ids = ws.assistants.assistants.filter(a => a.selected).map(a => a.id);
    await api.configureAssistants(ids);
  },
  roots:       async (ws, api) => {
    // Roots are persisted to the DB when the user clicks "Add" on the roots page.
    // Here we trigger scanning for any roots that are in DB but not yet scanned.
    for (const root of ws.roots.roots) {
      if (!root.scanned && root.path) await api.scanFolder(root.path);
    }
  },
  scan:        async () => {},
  projects:    async (ws, api) => {
    for (const p of ws.projects.projects) await api.updateProject(p.id, p);
  },
  libraries:   async () => {},
  instruments: async () => {},
  inference:   async () => {},
  assignments: async () => {},
  done:        async () => { await appState.setSetupComplete(); },
};

// ── WizardState ─────────────────────────────────────────────

function cloneStages(): WizardStage[] {
  return STAGES.map(s => ({ ...s }));
}

export class WizardState {
  // Single source of truth for stage metadata + persisted status + transient active.
  // The rail iterates this array; the page header indexes it.
  stages = $state<WizardStage[]>(cloneStages());

  preferences = $state<PreferencesData>({
    displayName: '', contributeLearnings: true, reviewBeforeShare: true,
    shareSchedule: 'weekly-saturday', downloadCollective: 'weekly',
    correctionAggressiveness: 'balanced', digestCadence: 'daily',
    nudgeOnRegression: true, anonymizedTelemetry: false, showWelcome: true,
  });
  assistants  = $state<AssistantsSlice>({ assistants: [] });
  roots       = $state<RootsSlice>({ roots: [], newPath: '' });
  scan        = $state<ScanSlice>({ baseline: null, started: false, done: false });
  projects    = $state<ProjectsSlice>({ projects: [] });
  libraries   = $state<LibrariesSlice>({ libs: [] });
  instruments = $state<InstrumentsSlice>({ mcps: [] });

  // ── Derived ──

  get firstPendingStage(): string {
    const pending = this.stages.find(s => s.status !== 'done');
    return pending ? pending.id : 'done';
  }

  get allDone(): boolean {
    return this.stages.every(s => s.status === 'done');
  }

  isStageComplete(id: string): boolean {
    return this.stages.find(s => s.id === id)?.status === 'done';
  }

  /** Mark a stage active, clearing any previous active stage. */
  setActive(id: string): void {
    for (const s of this.stages) s.active = s.id === id;
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

  async hydrate(data: WizardLoadData): Promise<void> {
    // Reset stages from the canonical static defs, then layer persisted status.
    this.stages = cloneStages();
    for (const s of this.stages) {
      if (data.completion[s.id] === 'done') s.status = 'done';
    }

    this.preferences = { ...data.preferences };

    // Prefill displayName from system username if empty
    if (!this.preferences.displayName) {
      const user = await guessUserName();
      if (user) this.preferences.displayName = user;
    }

    this.assistants = {
      assistants: data.assistantFamilies.map(a => ({
        ...a,
        selected: a.selected ?? a.variants.some(v => v.installed),
      })),
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
    const handler = COMMIT_HANDLERS[stageId];
    if (!handler) return true;
    try {
      await handler(this, api);
      await api.setConfig({ [`setup.${stageId}`]: 'done' });
      const stage = this.stages.find(s => s.id === stageId);
      if (stage) stage.status = 'done';
      return true;
    } catch {
      return false;
    }
  }
}

/** Best-effort username — async so it can call Tauri's homeDir(). */
async function guessUserName(): Promise<string> {
  try {
    const stored = typeof localStorage !== 'undefined'
      ? localStorage.getItem('sensei:userName') : null;
    if (stored) return stored;

    // In Tauri, homeDir() returns the real home directory (e.g. /Users/jerry)
    if (hasTauri()) {
      const { homeDir } = await import('@tauri-apps/api/path');
      const home = await homeDir();
      // Strip trailing slash, then take the last path segment
      const match = home.replace(/\/$/, '').match(/\/([^/]+)$/);
      if (match) {
        const name = match[1];
        return name.charAt(0).toUpperCase() + name.slice(1);
      }
    }

    // Browser fallback: try location.pathname (works outside Tauri)
    const pathMatch = (typeof location !== 'undefined' ? location.pathname : '')
      .match(/\/Users\/([^/]+)/);
    if (pathMatch) {
      const name = pathMatch[1];
      return name.charAt(0).toUpperCase() + name.slice(1);
    }
  } catch { /* ignore */ }
  return '';
}

export const wizardState = new WizardState();
