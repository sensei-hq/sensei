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
    await this.setConfig('setup_complete', '1');
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

  async load() {
    if (typeof localStorage !== 'undefined') {
      const stored = parseInt(localStorage.getItem('sensei:port') ?? '', 10);
      if (!isNaN(stored) && stored > 0) this.port = stored;
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
    this.config = {};
    this.loaded = false;

    // sensei:health is owned by HealthState — do not touch it here.
    if (typeof localStorage !== 'undefined') {
      const port = localStorage.getItem('sensei:port');
      localStorage.clear();
      if (port) localStorage.setItem('sensei:port', port);
    }
  }
}

/** Singleton instance — import and use directly. */
export const appState = new AppState();
