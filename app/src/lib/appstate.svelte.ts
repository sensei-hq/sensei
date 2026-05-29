/**
 * Application state — singleton class managing daemon config and port.
 * Only `sensei:port` and `sensei:setup-complete` stay in localStorage.
 * The health gate (`sensei:health` in sessionStorage) is owned by HealthState.
 */
import { senseiApi } from './api.js';
import { hasTauri } from './bootstrap.js';
import { healthState } from './health-state.svelte.js';
import { STORAGE_KEYS } from './storage-keys.js';

// Build-time port injected by vite.config.ts — 7745 for dev/debug, 7744 for prod.
// No async resolution needed; page loaders can read appState.port immediately.
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
    } catch { return []; }
  }

  get setupComplete(): boolean {
    // Daemon-canonical when in-memory cache is hydrated.
    if (this.config['setup_complete'] === '1') return true;
    // Cold-start sync gate. appState.load() runs from the observatory
    // layout's load() — but the reroute hook fires BEFORE that load on
    // the very first navigation of the app session, so config is still
    // {} and we'd incorrectly bounce to /setup/welcome. The localStorage
    // cache is written by wizardState.setCompleted() and reconciled by
    // appState.load() on every successful refresh, so it cannot drift
    // ahead of the daemon — only behind, which is the safe direction
    // for a "did setup already complete?" gate.
    if (this.loaded) return false;
    // Guarded localStorage access — vitest's node test env can leave a
    // partial `localStorage` global on the window without getItem,
    // which would throw on `.getItem()`.
    try {
      const ls = (globalThis as { localStorage?: Storage }).localStorage;
      if (!ls || typeof ls.getItem !== 'function') return false;
      return ls.getItem(STORAGE_KEYS.setupComplete) === '1';
    } catch {
      return false;
    }
  }

  get userName(): string {
    return this.config['user_name'] || '';
  }

  get dismissedSuggestions(): string[] {
    try {
      return JSON.parse(this.config['dismissed_suggestions'] ?? '[]');
    } catch { return []; }
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
   * Hydrate from the daemon (Tauri) or skip in browser-only dev mode.
   *
   * Returns `true` if appState is usable after this call — either daemon
   * config landed or we deliberately skipped the daemon. Returns `false`
   * only when we expected the daemon (Tauri mode) and couldn't reach it;
   * group layouts (`(observatory)/+layout.ts` etc.) translate that into
   * a 503 so non-health pages never silently mount with empty config.
   */
  async load(): Promise<boolean> {
    if (typeof localStorage !== 'undefined') {
      const stored = parseInt(localStorage.getItem(STORAGE_KEYS.port) ?? '', 10);
      if (!isNaN(stored) && stored > 0) this.port = stored;
    }

    // Browser (no Tauri) → skip daemon calls
    if (!hasTauri()) {
      this.config = {};
      this.loaded = true;
      return true;
    }

    const api = senseiApi(this.port);
    const result = await api.tryGetConfig();
    if (!result.ok) {
      // Daemon unreachable — leave the cache untouched so a transient
      // outage doesn't bounce the user back through setup. Don't mark
      // loaded — the caller decides whether to retry or surface 503.
      return false;
    }

    this.config = result.data;
    // Daemon is the canonical source for setup completion. Reconcile the
    // localStorage cache (the sync gate read by hooks.client.ts) so it
    // can never drift past a daemon write that didn't actually land.
    if (typeof localStorage !== 'undefined') {
      if (this.config['setup_complete'] === '1') {
        localStorage.setItem(STORAGE_KEYS.setupComplete, '1');
      } else {
        localStorage.removeItem(STORAGE_KEYS.setupComplete);
      }
    }
    this.loaded = true;
    return true;
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
