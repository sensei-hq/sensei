/**
 * Application state — singleton class managing daemon config and port.
 * Only `sensei:port` and `sensei:setup-complete` stay in localStorage.
 * `healthReady` is in-memory only — reset on every cold start so bootstrap
 * always runs and ensures daemon + postgres are up before the app is usable.
 */
import { senseiApi } from './api.js';

export class AppState {
  port = $state(7744);
  config = $state<Record<string, string>>({});
  loaded = $state(false);
  /**
   * True once bootstrap confirmed all gates ready in this session.
   * In-memory only — not persisted, so bootstrap runs on every cold start.
   */
  healthReady = $state(false);

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
    await this.setConfig('setup_complete', '1');
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('sensei:setup-complete', '1');
    }
  }

  /**
   * Mark health check as passed for this session.
   * Writes to sessionStorage (cleared on cold start by WKWebView) so the
   * reroute hook can read it synchronously without importing this module.
   * Also caches setup-complete to localStorage for the setup gate.
   */
  setHealthReady() {
    this.healthReady = true;
    if (typeof sessionStorage !== 'undefined') {
      sessionStorage.setItem('sensei:health', 'ready');
    }
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('sensei:setup-complete', this.setupComplete ? '1' : '0');
    }
  }

  async dismissSuggestion(id: string) {
    const current = this.dismissedSuggestions;
    if (!current.includes(id)) {
      await this.setConfig('dismissed_suggestions', JSON.stringify([...current, id]));
    }
  }

  async load() {
    if (typeof localStorage !== 'undefined') {
      const stored = parseInt(localStorage.getItem('sensei:port') ?? '', 10);
      if (!isNaN(stored) && stored > 0) this.port = stored;
      // healthReady is intentionally NOT loaded from storage — bootstrap must
      // run on every cold start to ensure services are up.
    }

    // Browser (no Tauri) → skip daemon calls
    if (typeof window !== 'undefined' && !(window as any).__TAURI__) {
      this.config = {};
      this.loaded = true;
      return;
    }

    const api = senseiApi(this.port);
    try {
      this.config = await api.getConfig();
    } catch {
      this.config = {};
    }

    this.loaded = true;
  }

  async reset() {
    try {
      await fetch(`http://127.0.0.1:${this.port}/api/reset`, { method: 'POST' });
    } catch { /* non-fatal */ }

    this.config = {};
    this.loaded = false;
    this.healthReady = false;

    if (typeof sessionStorage !== 'undefined') {
      sessionStorage.removeItem('sensei:health');
    }
    if (typeof localStorage !== 'undefined') {
      const port = localStorage.getItem('sensei:port');
      localStorage.clear();
      if (port) localStorage.setItem('sensei:port', port);
    }
  }
}

/** Singleton instance — import and use directly. */
export const appState = new AppState();
