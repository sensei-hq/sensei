/**
 * Application state — singleton class managing daemon config and port.
 * Only `sensei:port` and `sensei:setup-complete` stay in localStorage.
 * The health gate (`sensei:health` in sessionStorage) is owned by HealthState.
 */
import { senseiApi } from './api.js';

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
    return this.config['setup_complete'] === '1';
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
      localStorage.setItem('sensei:port', String(port));
    }
  }

  async setConfig(key: string, value: string) {
    this.config = { ...this.config, [key]: value };
    const api = senseiApi(this.port);
    await api.setConfig({ [key]: value });
  }

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

  async setSetupComplete() {
    // Optimistically update in-memory config so callers reading appState
    // immediately after this method see a consistent state on success.
    this.config = { ...this.config, setup_complete: '1' };

    // Daemon is canonical — only write the local cache if the daemon write
    // succeeded. Throwing on failure lets the wizard surface the error to
    // the user instead of silently passing the setup gate next launch.
    const api = senseiApi(this.port);
    const result = await api.trySetConfig({ setup_complete: '1' });
    if (!result.ok) {
      delete this.config['setup_complete'];
      this.config = { ...this.config };
      throw new Error(
        `Failed to mark setup complete on daemon: ${result.error.message}`,
      );
    }

    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('sensei:setup-complete', '1');
    }
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
      const stored = parseInt(localStorage.getItem('sensei:port') ?? '', 10);
      if (!isNaN(stored) && stored > 0) this.port = stored;
    }

    // Browser (no Tauri) → skip daemon calls
    if (typeof window !== 'undefined' && !(window as any).__TAURI__) {
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
      this.config = {};
      return false;
    }

    this.config = result.data;
    // Daemon is the canonical source for setup completion. Reconcile the
    // localStorage cache (the sync gate read by hooks.client.ts) so it
    // can never drift past a daemon write that didn't actually land.
    if (typeof localStorage !== 'undefined') {
      if (this.config['setup_complete'] === '1') {
        localStorage.setItem('sensei:setup-complete', '1');
      } else {
        localStorage.removeItem('sensei:setup-complete');
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
      const PROTECTED = ['sensei:port', 'sensei:app-version'] as const;
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
