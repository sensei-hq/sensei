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

// Build-time port injected by vite.config.ts (single value: 7744). Kept as a
// define so we don't hardcode 7744 at every fetch site; if the daemon ever
// moves, vite.config.ts is the only place to change.
declare const __SENSEI_DEFAULT_PORT__: number;
const DEFAULT_PORT = typeof __SENSEI_DEFAULT_PORT__ !== 'undefined' ? __SENSEI_DEFAULT_PORT__ : 7744;

export class AppState {
  port = $state(DEFAULT_PORT);
  config = $state<Record<string, string>>({});
  loaded = $state(false);

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

  async setPort(port: number) {
    this.port = port;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(STORAGE_KEYS.port, String(port));
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
    if (typeof localStorage !== 'undefined') {
      const stored = parseInt(localStorage.getItem(STORAGE_KEYS.port) ?? '', 10);
      if (!isNaN(stored) && stored > 0) this.port = stored;
    }

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

  async reset() {
    this.config = {};
    this.loaded = false;

    // sensei:health is owned by HealthState — do not touch it here.
    // The PROTECTED list captures localStorage keys that other subsystems
    // own (port discovery, the upgrader's staged-version flag, …). Clearing
    // them here would silently skip an in-flight upgrade or strand the port.
    if (typeof localStorage !== 'undefined') {
      const PROTECTED = [STORAGE_KEYS.port, STORAGE_KEYS.appVersion] as const;
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
