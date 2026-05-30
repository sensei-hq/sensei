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
import { healthState } from './health-state.svelte.js';
import { scanState } from './scan-state.svelte.js';
import { loadWizardData } from './setup/loaders.js';
import { STORAGE_KEYS } from './storage-keys.js';
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
  preferences: async (ws) => {
    // Daemon write + appState cache update in one call so the next reader
    // (e.g. greeting on the welcome page) sees the new user_name without
    // a fresh appState.load().
    await appState.setConfigs({
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
  roots:       async () => {
    // Roots are persisted to the DB at "Add" time on the roots page; this
    // stage's commit is now a no-op. Scan triggering moved to the Scan
    // stage (scanState.start) so SSE is subscribed before any work emits.
    // Without that ordering, the wave of events from scan_root would land
    // on a broadcast channel with no subscriber and the UI would show
    // "stuck at queued" for everything that finished pre-mount.
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
  libraries:   async (ws) => {
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
    await appState.setConfigs({ 'setup.libraries': JSON.stringify({ wrapped, disabled }) });
  },
  instruments: async (ws) => {
    // Persist the user's MCP selection to `setup.instruments`. Once a daemon
    // registry endpoint lands, the same key can drive install/uninstall.
    const selected = ws.instruments.mcps.filter(m => m.selected).map(m => m.id);
    const deselected = ws.instruments.mcps.filter(m => !m.selected).map(m => m.id);
    await appState.setConfigs({ 'setup.instruments': JSON.stringify({ selected, deselected }) });
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
  done:        async (ws) => { await ws.setCompleted(); },
};

// ── WizardState ─────────────────────────────────────────────

function cloneStages(): WizardStage[] {
  return STAGES.map(s => ({ ...s }));
}

/** Seed for `setupComplete` $state — reads localStorage[`setupComplete`]
 *  as the cold-start fallback so the reroute hook gets a stable answer
 *  before any async load(). Updated by load() (from daemon truth) and
 *  setCompleted() (when the user finishes the wizard). */
function initialSetupComplete(): boolean {
  // SvelteKit static prerender (adapter-static) shims `localStorage` as
  // an object whose methods are absent — so `typeof localStorage` is
  // 'object' but `localStorage.getItem` is undefined. Probe getItem
  // directly instead of trusting the global presence.
  const ls = (globalThis as { localStorage?: Storage }).localStorage;
  if (!ls || typeof ls.getItem !== 'function') return false;
  try { return ls.getItem(STORAGE_KEYS.setupComplete) === '1'; }
  catch (e) {
    console.warn('[wizardState] localStorage read failed at cold start; defaulting to false', e);
    return false;
  }
}

export class WizardState {
  /**
   * Canonical "is the wizard finished?" flag. Owned here (rather than on
   * appState) because completion is a wizard concern. appState surfaces
   * this through its `setupComplete` / `setupOk` facade.
   *
   * Lifecycle:
   *   - Initial value seeded from localStorage so reroute has a stable
   *     answer on cold start.
   *   - `load()` reconciles against daemon's `config['setup_complete']`
   *     on every invocation, and syncs localStorage to whatever the
   *     daemon reports. The module-level $effect.root below re-fires
   *     load() whenever healthState flips to ok (typically right after
   *     a resolve pass that may have recreated the DB), so a dropped DB
   *     can't leave the cache stuck at '1'.
   *   - `setCompleted()` flips it to true on the final wizard stage and
   *     writes '1' to localStorage.
   */
  setupComplete = $state<boolean>(initialSetupComplete());

  // Single source of truth for stage metadata + persisted status + transient active.
  // The rail iterates this array; the page header indexes it.
  stages = $state<WizardStage[]>(cloneStages());

  constructor() {
    // Reactive bridge from healthState. When health flips to ok (typically
    // right after a resolve pass that may have recreated the DB and its
    // config table), re-run load() so `setupComplete` + the localStorage
    // cache reconcile against fresh daemon truth — a dropped DB can't
    // leave the cache stuck at '1'.
    //
    // `$effect` is the right primitive here, NOT `$derived`: $derived is
    // synchronous and would store a Promise instead of the boolean.
    // The effect only re-fires when `healthState.isOk` actually changes,
    // so steady-state-ok doesn't re-trigger load.
    //
    // $effect.root creates a non-component tracking scope so this works
    // for a module-level singleton.
    $effect.root(() => {
      $effect(() => {
        // load() catches its own errors and returns false on failure —
        // it can't reject, so no .catch() is needed here. `void` makes
        // the fire-and-forget intent explicit (and satisfies the rule
        // against returning a Promise from an $effect callback, which
        // Svelte would mistake for a cleanup function).
        if (healthState.isOk) void this.load();
      });
    });
  }

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
   * the setup flow or the observatory. Reads our own `setupComplete`
   * field, which is the canonical owner of this state (see field docs).
   */
  get isOk(): boolean {
    return this.setupComplete;
  }

  isStageComplete(id: string): boolean {
    return this.stages.find(s => s.id === id)?.status === 'done';
  }

  /** Mark a stage active, clearing any previous active stage.
   *  Rebuilds the array so Svelte 5 derived signals re-evaluate —
   *  in-place mutation of plain-object items is not guaranteed to
   *  propagate through $derived chains. */
  setActive(id: string): void {
    // Idempotency guard — without this, the layout's $effect re-fires every
    // tick because the array rebuild creates new object references that the
    // $derived `stage = stages[currentIdx]` sees as changed, re-triggering
    // the effect that called us. Svelte then aborts with effect_depth_exceeded.
    const current = this.stages.find(s => s.active);
    if (current?.id === id) return;
    this.stages = this.stages.map(s => ({ ...s, active: s.id === id }));
  }

  canAdvance(stageId: string): boolean {
    switch (stageId) {
      case 'preferences': return this.preferences.displayName.trim().length > 0;
      case 'roots':       return this.roots.roots.length > 0;
      // Scan completion now lives on scanState (the live runtime singleton)
      // because the scan can finish while the user is on another stage; the
      // wizardState slice is just the persisted baseline snapshot.
      case 'scan':        return scanState.completed;
      default:            return true;
    }
  }

  // ── Lifecycle ──

  /**
   * Mark the wizard as completed end-to-end. Writes setup_complete to the
   * daemon and updates the appState cache + localStorage sync-gate so the
   * reroute hook sees the new state immediately on the next navigation.
   *
   * Lives here (not on appState) because completion is a wizard concern —
   * appState just exposes the resulting flag via .setupOk for callers that
   * want a status read without coupling to the wizard module.
   */
  async setCompleted(): Promise<void> {
    // setConfigs throws on daemon failure — the done-stage commit handler
    // re-raises so the wizard layout can show the error instead of
    // navigating to the observatory.
    await appState.setConfigs({ setup_complete: '1' });
    this.setupComplete = true;
    const ls = (globalThis as { localStorage?: Storage }).localStorage;
    if (ls && typeof ls.setItem === 'function') {
      ls.setItem(STORAGE_KEYS.setupComplete, '1');
    }
  }

  /** True once load() has resolved at least once. The (config) layout uses
   *  this as the gate before rendering — keeps wizard pages from flashing
   *  with empty state during the cold mount. */
  loaded = $state(false);

  /**
   * Fetch wizard data from the daemon and apply it. Single entry point —
   * components / layouts call this; nobody else should reach into the
   * daemon for wizard concerns. Mirrors `healthState.init()` and
   * `appState.load()` so all three globals share the same idempotent
   * load() + state-driven UI pattern.
   *
   * Returns false on daemon unreachable so the layout can surface a 503
   * instead of mounting with stale or empty state.
   */
  async load(): Promise<boolean> {
    try {
      const data = await loadWizardData(appState.port);
      await this.hydrate(data);
      this.loaded = true;
      return true;
    } catch {
      // Don't clear existing state — a transient daemon outage shouldn't
      // wipe the cache the user is currently looking at (mirrors the
      // contract appState.load() now follows after the comment/code
      // contradiction was resolved).
      return false;
    }
  }

  async hydrate(data: WizardLoadData): Promise<void> {
    // Wizard init resets the live scan runtime — a fresh wizard session
    // starts with an empty SSE stream + Begin button. The runtime survives
    // stage navigation within the session; only re-entering the wizard
    // (which is what triggers hydrate) clears it.
    scanState.reset();

    // Reconcile the canonical `setupComplete` flag against daemon truth
    // and propagate to localStorage. Single source of truth: this method
    // and `setCompleted()` are the only writers — anywhere else that
    // touches `localStorage[setupComplete]` is a layering violation.
    //
    // Guard the global access — vitest's node env can leave a partial
    // `localStorage` global on globalThis without setItem/removeItem
    // wired up; `typeof localStorage !== 'undefined'` alone isn't enough.
    this.setupComplete = data.setupComplete;
    const ls = (globalThis as { localStorage?: Storage }).localStorage;
    if (ls && typeof ls.setItem === 'function' && typeof ls.removeItem === 'function') {
      if (data.setupComplete) ls.setItem(STORAGE_KEYS.setupComplete, '1');
      else                    ls.removeItem(STORAGE_KEYS.setupComplete);
    }

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
   *
   * Filter libs to those touching at least one repo scanned in this wizard run
   * to avoid showing stale libs from the persistent DB.
   */
  async refreshLibraries(): Promise<void> {
    const api = senseiApi(appState.port);
    const fresh = await api.getLibs();

    // Set of repo (folder) names scanned in this wizard run. Filter libs to those
    // touching at least one of these so we don't show the entire persistent DB.
    const scannedRepoNames = new Set<string>();
    for (const p of this.projects.projects) {
      for (const f of p.folders) scannedRepoNames.add(f.name);
    }

    const previous = new Map(this.libraries.libs.map(l => [l.name, l.enabled]));
    this.libraries = {
      libs: fresh.libs
        .filter(l => (l.repos ?? []).some((r: string) => scannedRepoNames.has(r)))
        .map(l => ({
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

  /**
   * Commit the named stage's data to the daemon and mark the stage done.
   *
   * Throws on failure — the layout's commit handler catches and surfaces
   * the error message via `commitError`. Previously this method swallowed
   * errors for non-`done` stages and returned `false`, which left the
   * user with a stuck Continue button and no feedback. The aggregate
   * error message is additive to any per-card UI (e.g. assistants /
   * inference set their own per-row failure state in their handlers).
   *
   * Returns `void` on success — handler-less stages (welcome, roots,
   * scan) succeed trivially.
   */
  async commitStage(stageId: string): Promise<void> {
    const api = senseiApi(appState.port);
    const handler = COMMIT_HANDLERS[stageId];
    if (handler) {
      await handler(this, api);
      // Marker write goes through appState so the in-memory config picks
      // up the `setup.X: done` keys — that matters for reroute()'s setup
      // gate and any other reader of appState.config that runs before the
      // next load().
      await appState.setConfigs({ [`setup.${stageId}`]: 'done' });
    }
    const stage = this.stages.find(s => s.id === stageId);
    if (stage) stage.status = 'done';
  }
}

/** Best-effort username — async so it can call Tauri's homeDir().
 *  Each external boundary (localStorage, Tauri native, location) is
 *  wrapped separately so a failure in one source falls through to the
 *  next instead of aborting the whole guess. Failures are logged so
 *  they don't go silent. */
async function guessUserName(): Promise<string> {
  // localStorage boundary — can throw in Safari private mode or under
  // quota pressure. typeof guard handles SSR/test envs where it's undefined.
  if (typeof localStorage !== 'undefined') {
    try {
      const stored = localStorage.getItem(STORAGE_KEYS.userName);
      if (stored) return stored;
    } catch (e) {
      console.warn('guessUserName: localStorage read failed', e);
    }
  }

  // Tauri native boundary — homeDir() can throw if the path plugin isn't
  // registered, and the dynamic import can fail if the bundle is stale.
  // Don't bail out of the guess on failure — fall through to the browser
  // pathname heuristic.
  if (hasTauri()) {
    try {
      const { homeDir } = await import('@tauri-apps/api/path');
      const home = await homeDir();
      const match = home.replace(/\/$/, '').match(/\/([^/]+)$/);
      if (match) return match[1].charAt(0).toUpperCase() + match[1].slice(1);
    } catch (e) {
      console.warn('guessUserName: Tauri homeDir() failed', e);
    }
  }

  // Browser fallback — pure string ops, can't throw.
  const pathMatch = (typeof location !== 'undefined' ? location.pathname : '')
    .match(/\/Users\/([^/]+)/);
  if (pathMatch) return pathMatch[1].charAt(0).toUpperCase() + pathMatch[1].slice(1);

  return '';
}

export const wizardState = new WizardState();
