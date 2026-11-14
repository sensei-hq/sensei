/**
 * Application state — singleton class managing daemon config and port.
 * Only `sensei:port` and `sensei:setup-complete` stay in localStorage.
 * The health gate (`sensei:health` in sessionStorage) is owned by HealthState.
 */
import type { PanelState } from '@rokkit/app';
import { failureReason, senseiApi } from './api.js';
import { hasTauri } from './bootstrap.js';
import { healthState } from './health-state.svelte.js';
import { wizardState } from './wizard-state.svelte.js';
import { browserStorage, STORAGE_KEYS } from './storage-keys.js';

// Daemon port. Hardcoded — there is one daemon, one port, one source of
// truth. The previous indirection (a vite-define + a localStorage override)
// added two failure modes (stale localStorage value, stale bundle) without
// any callsite that actually needed runtime variability.
const DEFAULT_PORT = 7744;

/** The one config entry that remembers every screen's panels (D87 §4). */
export const VIEWS_KEY = 'ui.views';

const PANEL_IDS = ['info', 'stats', 'filters', 'side'] as const;
type PanelId = (typeof PANEL_IDS)[number];

function isPanelId(key: string): key is PanelId {
  return (PANEL_IDS as readonly string[]).includes(key);
}

/** The diagrams' old per-band localStorage keys, and the panel each one became. */
const LEGACY_VIEW_KEYS: ReadonlyArray<readonly [string, PanelId]> = [
  [STORAGE_KEYS.diagramsAbout, 'info'],
  [STORAGE_KEYS.diagramsStats, 'stats'],
  [STORAGE_KEYS.diagramsControls, 'filters'],
  [STORAGE_KEYS.diagramsSide, 'side'],
];

/**
 * `ui.views` → the stored panels, one entry per view id, plus what could not be
 * read. An entry holds the four panel ids, boolean, and nothing else; anything
 * else is dropped and said, while the rest of the map is kept. A value that is
 * not JSON is never read as an empty map silently: the error comes back with
 * it, so the shell can say why a view opened all open.
 */
function readViews(raw: string | undefined): { views: Record<string, PanelState>; error: string | null } {
  if (raw === undefined) return { views: {}, error: null };
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (e) {
    return { views: {}, error: `ui.views could not be read: ${e instanceof Error ? e.message : String(e)}` };
  }
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
    return { views: {}, error: 'ui.views could not be read: it is not a map of views' };
  }
  const views: Record<string, PanelState> = {};
  let invalid = false;
  for (const [viewId, entry] of Object.entries(parsed as Record<string, unknown>)) {
    if (entry === null || typeof entry !== 'object' || Array.isArray(entry)) {
      invalid = true;
      continue;
    }
    const panels: PanelState = {};
    for (const [key, value] of Object.entries(entry as Record<string, unknown>)) {
      if (!isPanelId(key) || typeof value !== 'boolean') {
        invalid = true;
        continue;
      }
      panels[key] = value;
    }
    views[viewId] = panels;
  }
  return {
    views,
    error: invalid ? 'ui.views could not be read: entries that are not panel states were dropped' : null,
  };
}

export class AppState {
  /** Daemon port. Build-time constant — see DEFAULT_PORT above for why
   *  this is not a runtime $state and not read from localStorage. */
  readonly port = DEFAULT_PORT;
  config = $state<Record<string, string>>({});
  loaded = $state(false);
  /** Why the last config read failed, or null. The config keeps its last good value. */
  configError = $state<string | null>(null);
  #retry: ReturnType<typeof setTimeout> | null = null;

  /** Cached workspace count for the observatory rail badge. Workspaces change
   *  rarely, so this is fetched once (on layout mount via loadWorkspaceCount)
   *  and reused — the rail reads this rather than re-fetching on every
   *  render. `null` until loaded → the badge stays hidden. */
  workspaceCount = $state<number | null>(null);

  /** Every view's remembered panels, read once from `ui.views` and kept whole —
   *  a save writes the entire map back, so this cache must never be written
   *  from before the daemon's map has been read. */
  #views = $state<Record<string, PanelState>>({});
  /** Whether a config read has succeeded at least once, so `#views` is the
   *  daemon's map rather than an empty cache. The whole-map write needs it. */
  #viewsRead = false;
  /** Why the views could not be read, or a save could not be kept, or null. */
  viewsError = $state<string | null>(null);

  /** Called with the keys of each config write the daemon stored, so the other
   *  windows (each holds its own copy) can re-read it. Set by the root layout. */
  onConfigSaved: ((keys: string[]) => void) | null = null;

  get activeWorkspaceId(): string | null {
    return this.config['active_workspace'] || this.config['active_solution'] || null;
  }

  get sidebarMaxItems(): number {
    return parseInt(this.config['sidebar_max_items'] ?? '5', 10);
  }

  get globalSkills(): string[] {
    try {
      return JSON.parse(this.config['global_skills'] ?? '["zero-errors-policy","managing-workspace-sessions","pattern-based-development"]');
    } catch (e) {
      console.warn('[appState] global_skills JSON.parse failed; returning empty list', e);
      return [];
    }
  }

  /** Facade passthrough — `setupComplete` is owned by wizardState (the
   *  wizard is what completes setup, and what reconciles against the
   *  daemon's `setup_complete` config). appState surfaces it here so
   *  callers can read app status from one place. */
  get setupComplete(): boolean {
    return wizardState.setupComplete;
  }

  get userName(): string {
    return this.config['user_name'] || '';
  }

  get dismissedSuggestions(): string[] {
    try {
      return JSON.parse(this.config['dismissed_suggestions'] ?? '[]');
    } catch (e) {
      console.warn('[appState] dismissed_suggestions JSON.parse failed; returning empty list', e);
      return [];
    }
  }

  /** One key; the cache keeps its old value, and this throws, when the
   *  daemon does not store it (#304). */
  async setConfig(key: string, value: string) {
    await this.setConfigs({ [key]: value });
  }

  /**
   * Batch config write — daemon write + cache update in one call. Use this
   * from state classes (e.g. wizardState commit handlers) instead of calling
   * `senseiApi(...).setConfig({...})` directly, so the in-memory cache and
   * daemon stay in sync without a follow-up load(). Values may be strings
   * (raw) or any JSON-serialisable object — passed through as-is.
   */
  async setConfigs(map: Record<string, string>) {
    // Optimistic-with-rollback. A write the daemon did not store must not
    // stay in the cache: the next load() refetches daemon truth, which would
    // diverge from it, and a downstream gate (e.g. the boot gate's
    // setup_complete check) would read the wrong value — the cause of the
    // "Enter observatory → bounced back to /setup/welcome" symptom.
    const prev = this.config;
    this.config = { ...prev, ...map };
    const api = senseiApi(this.port);
    const result = await api.trySetConfig(map);
    if (!result.ok) {
      this.config = prev;
      throw new Error(
        `Daemon config write failed (${result.error.status}): ${result.error.message}`,
      );
    }
    this.onConfigSaved?.(Object.keys(map));
  }

  // ── View panels (D87 §4) ─────────────────────────────────────────────
  // One entry, `ui.views`, holds every screen's remembered bands. A view with
  // no entry is a first visit: everything open. The shell stores through here.

  /** The panels stored for a view; a first visit is `{}` — every band open. */
  viewPanels(viewId: string): PanelState {
    return this.#views[viewId] ?? {};
  }

  /** Keep one view's panels, writing the whole map so the other views survive.
   *  Refused before the daemon's map has been read, throws when the daemon
   *  refuses, and the cache then still holds what was really stored. */
  async setViewPanels(viewId: string, next: PanelState): Promise<void> {
    if (!this.#viewsRead) {
      throw new Error("The views haven't been read yet: refusing to overwrite them.");
    }
    const whole = { ...this.#views, [viewId]: next };
    try {
      await this.setConfigs({ [VIEWS_KEY]: JSON.stringify(whole) });
    } catch (e) {
      this.viewsError = e instanceof Error ? e.message : String(e);
      throw e;
    }
    this.#views = whole;
    this.viewsError = null;
  }

  /**
   * One-time: the diagrams' four localStorage toggles become `ui.views`, in
   * every diagram view id given (the old toggles were shared by all of them),
   * then the keys are deleted. A view that already has an entry is never
   * overwritten. Nothing to write is still a finished migration: once the keys
   * are read they are deleted, or they would be read on every start. A refused
   * write keeps them, so the next start migrates again.
   */
  async migrateLegacyViews(viewIds: string[]): Promise<void> {
    if (!this.#viewsRead) {
      throw new Error("The views haven't been read yet: refusing to migrate them.");
    }
    const storage = browserStorage();
    if (!storage) return;
    const present = LEGACY_VIEW_KEYS.filter(([key]) => storage.getItem(key) !== null);
    if (present.length === 0) return;
    const migrated: PanelState = {};
    for (const [key, panel] of present) migrated[panel] = storage.getItem(key) !== 'closed';
    const whole: Record<string, PanelState> = { ...this.#views };
    let wrote = false;
    for (const viewId of viewIds) {
      if (viewId in whole) continue;
      whole[viewId] = migrated;
      wrote = true;
    }
    if (wrote) {
      try {
        await this.setConfigs({ [VIEWS_KEY]: JSON.stringify(whole) });
      } catch (e) {
        this.viewsError = e instanceof Error ? e.message : String(e);
        throw e;
      }
      this.#views = whole;
    }
    for (const [key] of present) storage.removeItem(key);
  }

  // ── Facade passthroughs ──────────────────────────────────────────────
  // Encapsulate cross-global status so consumers can read app status from
  // one object rather than importing health/wizard/scan singletons
  // individually. Keep the underlying globals as the source of truth —
  // these are read-only views, not duplicates.

  get healthOk(): boolean { return healthState.isOk; }
  get setupOk():  boolean { return this.setupComplete; }

  async setActiveWorkspaceId(id: string | null) {
    if (id) {
      await this.setConfig('active_workspace', id);
    } else {
      // Same rollback as setConfigs (#304): a clear the daemon refused keeps
      // the active workspace in the cache, and the caller hears why.
      const prev = this.config;
      const { active_workspace: _cleared, ...rest } = prev;
      this.config = rest;
      const result = await senseiApi(this.port).tryDeleteConfig('active_workspace');
      if (!result.ok) {
        this.config = prev;
        throw new Error(
          `Daemon config delete failed (${result.error.status}): ${result.error.message}`,
        );
      }
    }
  }

  async setSidebarMaxItems(val: number) {
    await this.setConfig('sidebar_max_items', String(Math.max(1, Math.min(20, val))));
  }

  async setGlobalSkills(skills: string[]) {
    await this.setConfig('global_skills', JSON.stringify(skills));
  }

  async dismissSuggestion(id: string) {
    const current = this.dismissedSuggestions;
    if (!current.includes(id)) {
      await this.setConfig('dismissed_suggestions', JSON.stringify([...current, id]));
    }
  }

  /**
   * Hydrate from the daemon. Always succeeds — if the daemon is unreachable
   * we log and fall back to an empty config so callers never have to deal
   * with a "load failed" branch. The routing layer (the boot gate (routes/+layout.ts)) is
   * the canonical place that decides what to show when health isn't ok; it
   * will send the user to /health, where the bootstrap resolver chain heals
   * whatever's broken and brings the user back. This module just provides
   * a usable cache.
   */
  async load(): Promise<void> {
    // Browser (no Tauri) → skip daemon calls and fall through to defaults.
    if (!hasTauri()) {
      this.config = {};
      this.loaded = true;
      return;
    }

    const api = senseiApi(this.port);
    const result = await api.tryGetConfig();
    if (!result.ok) {
      // Daemon unreachable — a restart refuses reads for a few seconds. The
      // config keeps its last good value (never {} — that reset every
      // preference in the window), the error is said, and the read is retried
      // once shortly after. Routing stays the boot gate's (routes/+layout.ts):
      // if health isn't ok it bounces to /health.
      this.configError = failureReason(result.error);
      this.loaded = true;
      this.#retry ??= setTimeout(() => {
        this.#retry = null;
        void this.load();
      }, 2_000);
      return;
    }

    this.configError = null;
    this.config = result.data;
    const views = readViews(result.data[VIEWS_KEY]);
    this.#views = views.views;
    this.viewsError = views.error;
    this.#viewsRead = true;
    // `setup_complete` ↔ localStorage reconciliation lives in
    // wizardState.hydrate() now (single owner). appState just caches the
    // config map; the boot gate reads `setupComplete` through the facade.
    this.loaded = true;
  }

  /**
   * Fetch + cache the workspace count for rail badges. No-op without Tauri
   * (browser dev). A failed read leaves the count unknown (null — no badge),
   * never 0, which read as "no workspaces" (#288); the rail must never block.
   */
  async loadWorkspaceCount(): Promise<void> {
    if (!hasTauri()) return;
    const read = await senseiApi(this.port).tryListWorkspaces();
    if (read.ok) this.workspaceCount = read.data.length;
  }

  async reset() {
    this.config = {};
    this.loaded = false;
    this.workspaceCount = null;

    // sensei:health is owned by HealthState — do not touch it here.
    // sensei:app-version is owned by the upgrader so an in-flight staged
    // upgrade survives reset.
    if (typeof localStorage !== 'undefined') {
      const PROTECTED = [STORAGE_KEYS.appVersion] as const;
      const preserved = PROTECTED
        .map((k) => [k, localStorage.getItem(k)] as const)
        .filter(([, v]) => v !== null);
      localStorage.clear();
      for (const [k, v] of preserved) localStorage.setItem(k, v as string);
    }
  }
}

/** Singleton instance — import and use directly. */
export const appState = new AppState();
