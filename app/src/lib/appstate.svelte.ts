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

  /** Cached project count for the observatory rail badge. Projects change
   *  rarely, so this is fetched once (on layout mount via loadProjectCount)
   *  and reused — the rail reads this rather than re-fetching on every
   *  render. `null` until loaded → the badge stays hidden. */
  projectCount = $state<number | null>(null);

  get activeProjectId(): string | null {
    return this.config['active_project'] || this.config['active_solution'] || null;
  }

  get sidebarMaxItems(): number {
    return parseInt(this.config['sidebar_max_items'] ?? '5', 10);
  }

  get globalSkills(): string[] {
    try {
      return JSON.parse(this.config['global_skills'] ?? '["zero-errors-policy","managing-project-sessions","pattern-based-development"]');
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

  async setConfig(key: string, value: string) {
    this.config = { ...this.config, [key]: value };
    const api = senseiApi(this.port);
    await api.setConfig({ [key]: value });
  }

  /**
   * Batch config write — daemon write + cache update in one call. Use this
   * from state classes (e.g. wizardState commit handlers) instead of calling
   * `senseiApi(...).setConfig({...})` directly, so the in-memory cache and
   * daemon stay in sync without a follow-up load(). Values may be strings
   * (raw) or any JSON-serialisable object — passed through as-is.
   */
  async setConfigs(map: Record<string, string>) {
    // Optimistic-with-rollback. CRUCIAL: use trySetConfig (Result-returning)
    // instead of setConfig (the helper-level swallow that returns void on
    // every failure including 5xx and network errors). Without trySetConfig
    // a transient daemon error silently drops the write, the next load()
    // refetches daemon truth — which now diverges from our cache — and
    // any downstream gate (e.g. reroute's setup_complete check) reads the
    // wrong value. This was the cause of the "Enter observatory → bounced
    // back to /setup/welcome" symptom.
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

  async setActiveProjectId(id: string | null) {
    if (id) {
      await this.setConfig('active_project', id);
    } else {
      delete this.config['active_project'];
      this.config = { ...this.config };
      const api = senseiApi(this.port);
      await api.deleteConfig('active_project');
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
   * with a "load failed" branch. The routing layer (hooks.ts::reroute) is
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
      // don't reroute — hooks::reroute is the single source of truth for
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
    // config map; reroute reads `setupComplete` through the facade.
    this.loaded = true;
  }

  /**
   * Fetch + cache the project count for rail badges. No-op without Tauri
   * (browser dev). Errors surface as an empty list (the api `get` fallback),
   * leaving the count at 0 rather than throwing — the rail must never block.
   */
  async loadProjectCount(): Promise<void> {
    if (!hasTauri()) return;
    const projects = await senseiApi(this.port).listProjects();
    this.projectCount = projects.length;
  }

  async reset() {
    this.config = {};
    this.loaded = false;
    this.projectCount = null;

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
