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

/** Per-family configuration progress during commit. */
export type AssistantConfigureState = 'idle' | 'configuring' | 'removing' | 'failed' | 'skipped';

/** True when every installed variant of a family is currently configured. */
export function familyIsConfigured(family: DaemonAssistantFamily): boolean {
  const installed = family.variants.filter(v => v.installed);
  return installed.length > 0 && installed.every(v => v.configured);
}

export interface AssistantsSlice {
  assistants: DaemonAssistantFamily[];
  /** Configure status per family id — updated live while Continue is in flight. */
  configureState: Record<string, AssistantConfigureState>;
  /** Error message per family id when configureState is 'failed'. */
  configureError: Record<string, string>;
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
  /** Per-project confirmation flag — true when the user has reviewed/approved the project. */
  confirmed: Record<string, boolean>;
}

export interface LibrariesSlice {
  libs: DaemonLibEntry[];
}

export interface InstrumentsSlice {
  mcps: DaemonMcpEntry[];
}

export type RouterSaveState = 'idle' | 'saving' | 'done' | 'failed';

export interface RouterEntry {
  id: string;
  name: string;
  providers: string[];
  capabilities: string[];
  needsKey: boolean;
  configured: boolean;
  draftKey: string;          // never sent until commit
  saveState: RouterSaveState;
  saveError: string;
}

export interface InferenceSlice {
  routers: RouterEntry[];
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
    // Reconcile user intent (switch state) with daemon truth (variant.configured)
    // family-by-family so the UI can show per-card progress:
    //   selected=true,  was configured=false → configure  (POST /api/assistants/configure)
    //   selected=false, was configured=true  → remove     (POST /api/assistants/remove)
    //   selected==was-configured             → no-op
    //
    // Daemon endpoints take *variant* ids (claude-code, claude-desktop), so each
    // family flows through with the list of its installed variants. On success,
    // we mutate variant.configured locally to keep the UI in sync without
    // refetching — daemon is canonical on next hydrate.
    const failed: string[] = [];

    for (const family of ws.assistants.assistants) {
      const variantIds = family.variants.filter(v => v.installed).map(v => v.id);
      if (variantIds.length === 0) {
        if (family.selected) ws.assistants.configureState[family.id] = 'skipped';
        continue;
      }
      const wasConfigured = familyIsConfigured(family);

      if (family.selected && !wasConfigured) {
        ws.assistants.configureState[family.id] = 'configuring';
        delete ws.assistants.configureError[family.id];
        try {
          const result = await api.configureAssistants(variantIds);
          if (result.errors.length > 0) {
            ws.assistants.configureState[family.id] = 'failed';
            ws.assistants.configureError[family.id] = result.errors.join('; ');
            failed.push(family.id);
          } else {
            for (const v of family.variants) if (v.installed) v.configured = true;
            ws.assistants.configureState[family.id] = 'idle';
          }
        } catch (e) {
          ws.assistants.configureState[family.id] = 'failed';
          ws.assistants.configureError[family.id] = e instanceof Error ? e.message : String(e);
          failed.push(family.id);
        }
      } else if (!family.selected && wasConfigured) {
        ws.assistants.configureState[family.id] = 'removing';
        delete ws.assistants.configureError[family.id];
        try {
          const result = await api.removeAssistants(variantIds);
          if (result.errors.length > 0) {
            ws.assistants.configureState[family.id] = 'failed';
            ws.assistants.configureError[family.id] = result.errors.join('; ');
            failed.push(family.id);
          } else {
            for (const v of family.variants) v.configured = false;
            ws.assistants.configureState[family.id] = 'idle';
          }
        } catch (e) {
          ws.assistants.configureState[family.id] = 'failed';
          ws.assistants.configureError[family.id] = e instanceof Error ? e.message : String(e);
          failed.push(family.id);
        }
      }
      // else: switch already matches daemon state, nothing to do
    }

    if (failed.length > 0) {
      throw new Error(`Failed to update: ${failed.join(', ')}`);
    }
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
    // Send only confirmed projects. The daemon's update_solution endpoint
    // persists name/description; per-folder role mutations go through the
    // dedicated /api/folders/{id} endpoint so a role change doesn't have
    // to round-trip through project metadata.
    for (const p of ws.projects.projects) {
      if (ws.projects.confirmed[p.id] === false) continue;
      await api.updateProject(p.id, { name: p.name, description: p.description });
      for (const folder of p.folders) {
        await api.updateFolder(folder.id, { role: folder.role });
      }
    }
  },
  libraries:   async (ws, api) => {
    // Persist the wrapped/disabled split to `setup.libraries`. The daemon
    // reads this when deciding which libs to index/wrap. Both lists are
    // stored explicitly so that adding a new lib after this commit defaults
    // to enabled (it's neither on `wrapped` nor `disabled` yet — see the
    // loader's mapLibraries).
    const wrapped: string[] = [];
    const disabled: string[] = [];
    for (const lib of ws.libraries.libs) {
      (lib.enabled ? wrapped : disabled).push(lib.name);
    }
    await api.setConfig({ 'setup.libraries': JSON.stringify({ wrapped, disabled }) });
  },
  instruments: async (ws, api) => {
    // Persist the user's MCP selection to `setup.instruments`. Once a daemon
    // registry endpoint lands, the same key can drive install/uninstall.
    const selected = ws.instruments.mcps.filter(m => m.selected).map(m => m.id);
    const deselected = ws.instruments.mcps.filter(m => !m.selected).map(m => m.id);
    await api.setConfig({ 'setup.instruments': JSON.stringify({ selected, deselected }) });
  },
  inference: async (ws, api) => {
    // Persist any non-empty drafted keys. Per-card status updates so
    // the UI shows progress. Failures are non-fatal — user can retry
    // later from settings — but we surface them via saveError.
    for (const router of ws.inference.routers) {
      if (!router.needsKey || router.draftKey.trim().length === 0) continue;
      router.saveState = 'saving';
      router.saveError = '';
      const result = await api.setGatewayRouterKey(router.id, router.draftKey.trim());
      if (result.ok) {
        router.configured = result.data.configured;
        router.draftKey = '';
        router.saveState = 'done';
      } else {
        router.saveState = 'failed';
        router.saveError = result.error.message;
      }
    }
  },
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
  assistants  = $state<AssistantsSlice>({ assistants: [], configureState: {}, configureError: {} });
  roots       = $state<RootsSlice>({ roots: [], newPath: '' });
  scan        = $state<ScanSlice>({ baseline: null, started: false, done: false });
  projects    = $state<ProjectsSlice>({ projects: [], confirmed: {} });
  libraries   = $state<LibrariesSlice>({ libs: [] });
  instruments = $state<InstrumentsSlice>({ mcps: [] });
  inference   = $state<InferenceSlice>({ routers: [] });

  // ── Derived ──

  get firstPendingStage(): string {
    const pending = this.stages.find(s => s.status !== 'done');
    return pending ? pending.id : 'done';
  }

  get allDone(): boolean {
    return this.stages.every(s => s.status === 'done');
  }

  /**
   * Mirror of healthState.isOk — true when the user has finished the setup
   * wizard. Read by hooks.reroute to decide whether the user should be in
   * the setup flow or the observatory. Daemon's `setup_complete` config key
   * is the canonical source; appState surfaces it as a sync getter.
   */
  get isOk(): boolean {
    return appState.setupComplete;
  }

  isStageComplete(id: string): boolean {
    return this.stages.find(s => s.id === id)?.status === 'done';
  }

  /** Mark a stage active, clearing any previous active stage.
   *  Rebuilds the array so Svelte 5 derived signals re-evaluate —
   *  in-place mutation of plain-object items is not guaranteed to
   *  propagate through $derived chains. */
  setActive(id: string): void {
    this.stages = this.stages.map(s => ({ ...s, active: s.id === id }));
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

    // Default selection: if the family is already configured on the daemon,
    // start with it selected (so the switch reflects daemon truth). If it's
    // installed but not configured, also start selected so the user's first
    // pass through configures it. Uninstalled families start unselected.
    this.assistants = {
      assistants: data.assistantFamilies.map(a => ({
        ...a,
        selected: a.selected ?? (a.variants.some(v => v.installed)),
      })),
      configureState: {},
      configureError: {},
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

    // Each loaded project starts as confirmed — the user has the option to
    // unconfirm via the Projects stage, in which case it is excluded from commit.
    this.projects = {
      projects: data.projects.map(p => ({ ...p, folders: p.folders ?? [] })),
      confirmed: Object.fromEntries(data.projects.map(p => [p.id, true])),
    };
    this.libraries = { libs: [...data.libraries.libs] };
    this.instruments = { mcps: [...data.mcps] };
    this.inference = {
      routers: data.routers.map(r => ({
        id: r.id,
        name: r.name,
        providers: r.providers,
        capabilities: r.capabilities,
        needsKey: r.needs_key,
        configured: r.configured,
        draftKey: '',
        saveState: 'idle' as RouterSaveState,
        saveError: '',
      })),
    };
  }

  /**
   * Re-fetch instruments (MCPs) from daemon, preserving the user's local
   * selection for entries that survive the refresh. The daemon recomputes
   * `recommended` per request based on the detected stack, so we re-honour
   * its truth except where the user explicitly toggled.
   */
  async refreshInstruments(): Promise<void> {
    const api = senseiApi(appState.port);
    const fresh = await api.listInstruments();
    const previous = new Map(this.instruments.mcps.map(m => [m.id, m.selected]));
    this.instruments = {
      mcps: fresh.mcps.map(m => ({
        ...m,
        selected: previous.get(m.id) ?? m.selected,
      })),
    };
  }

  /**
   * Re-fetch libraries from daemon, preserving the user's local enable/disable
   * intent for libs that survive the refresh. Daemon discovers libs during
   * scan, so the initial wizard hydrate may run before any libs exist.
   */
  async refreshLibraries(): Promise<void> {
    const api = senseiApi(appState.port);
    const fresh = await api.getLibs();
    const previous = new Map(this.libraries.libs.map(l => [l.name, l.enabled]));
    this.libraries = {
      libs: fresh.libs.map(l => ({
        id: l.id || l.name,
        name: l.name,
        ecosystem: l.ecosystem ?? '',
        version: l.version ?? null,
        description: l.description ?? null,
        pageCount: l.pageCount ?? 0,
        repos: l.repos ?? [],
        repoCount: l.repoCount ?? (l.repos?.length ?? 0),
        enabled: previous.get(l.name) ?? true,
      })),
    };
  }

  /**
   * Re-fetch gateway routers from daemon, preserving per-card draft state
   * (draftKey, saveState, saveError) for entries that survive the refresh.
   */
  async refreshInferenceRouters(): Promise<void> {
    const api = senseiApi(appState.port);
    const fresh = await api.listGatewayRouters();
    const previous = new Map(this.inference.routers.map(r => [r.id, r]));
    this.inference = {
      routers: fresh.routers.map(r => {
        const prev = previous.get(r.id);
        return {
          id: r.id,
          name: r.name,
          providers: r.providers,
          capabilities: r.capabilities,
          needsKey: r.needs_key,
          configured: r.configured,
          draftKey: prev?.draftKey ?? '',
          saveState: prev?.saveState ?? 'idle',
          saveError: prev?.saveError ?? '',
        };
      }),
    };
  }

  /**
   * Clear the stored API key for a gateway router and refresh the list.
   */
  async clearInferenceRouterKey(id: string): Promise<void> {
    const api = senseiApi(appState.port);
    await api.clearGatewayRouterKey(id);
    await this.refreshInferenceRouters();
  }

  /**
   * Re-fetch projects from daemon, merging in current confirmation state.
   * Called by the Projects page on mount because the daemon discovers projects
   * during scan — the layout's initial hydrate may have run before that.
   */
  async refreshProjects(): Promise<void> {
    const api = senseiApi(appState.port);
    const fresh = await api.listProjects();
    const previous = this.projects.confirmed;
    this.projects = {
      projects: fresh.map(p => ({ ...(p as unknown as DaemonProject), folders: ((p as unknown as DaemonProject).folders) ?? [] })),
      confirmed: Object.fromEntries(fresh.map(p => [p.id, previous[p.id] ?? true])),
    };
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
