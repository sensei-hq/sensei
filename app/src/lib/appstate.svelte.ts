/**
 * Application state — singleton class managing daemon config and port.
 * Only `sensei:port` and `sensei:setup-complete` stay in localStorage.
 * The health gate (`sensei:health` in sessionStorage) is owned by HealthState.
 */
import { senseiApi } from './api.js';
import { hasTauri } from './bootstrap.js';
import { healthState } from './health-state.svelte.js';
import { wizardState } from './wizard-state.svelte.js';
import { STORAGE_KEYS } from './storage-keys.js';

// Daemon port. Hardcoded — there is one daemon, one port, one source of
// truth. The previous indirection (a vite-define + a localStorage override)
// added two failure modes (stale localStorage value, stale bundle) without
// any callsite that actually needed runtime variability.
const DEFAULT_PORT = 7744;

export class AppState {
  /** Daemon port. Build-time constant — see DEFAULT_PORT above for why
   *  this is not a runtime $state and not read from localStorage. */
  readonly port = DEFAULT_PORT;
  config = $state<Record<string, string>>({});
  loaded = $state(false);

  /** Cached workspace count for the observatory rail badge. Workspaces change
   *  rarely, so this is fetched once (on layout mount via loadWorkspaceCount)
   *  and reused — the rail reads this rather than re-fetching on every
   *  render. `null` until loaded → the badge stays hidden. */
  workspaceCount = $state<number | null>(null);

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
      // Daemon unreachable — fall back to empty defaults. Don't throw,
      // don't redirect — the boot gate (routes/+layout.ts) is the single source of truth for
      // routing decisions; if health isn't ok it'll bounce to /health,
      // where the resolver chain handles the actual repair.
      console.warn('[appState] load() falling back to defaults — daemon-config fetch failed', result.error);
      this.config = {};
      this.loaded = true;
      return;
    }

    this.config = result.data;
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
