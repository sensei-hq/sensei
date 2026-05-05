/**
 * WizardState — singleton managing all setup wizard data.
 *
 * Hydrated from daemon via layout load. Stages read/write slices.
 * commitStage() persists to daemon on advance.
 */

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

// ── Stage order ─────────────────────────────────────────────

const STAGE_ORDER = [
  'welcome', 'preferences', 'assistants', 'roots', 'scan',
  'projects', 'libraries', 'instruments',
  'inference', 'assignments', 'done',
] as const;

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
    for (const root of ws.roots.roots) {
      if (!root.scanned) await api.scanFolder(root.path);
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

export class WizardState {
  completion = $state<Record<string, 'pending' | 'done'>>({});

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

  async hydrate(data: WizardLoadData): Promise<void> {
    this.completion = { ...data.completion };
    this.preferences = { ...data.preferences };

    // Prefill displayName from system username if empty
    if (!this.preferences.displayName) {
      const user = await guessUserName();
      if (user) this.preferences.displayName = user;
    }

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
    const handler = COMMIT_HANDLERS[stageId];
    if (!handler) return true;
    try {
      await handler(this, api);
      await api.setConfig({ [`setup.${stageId}`]: 'done' });
      this.completion[stageId] = 'done';
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
    if (typeof window !== 'undefined' && (window as any).__TAURI__) {
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
