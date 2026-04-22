/**
 * Centralized app state — replaces all localStorage usage.
 * Only `sensei:port` remains in localStorage (needed to find the daemon).
 * Everything else is stored on the daemon via /api/config.
 */
import { senseiApi } from './api.js';

// ── State ────────────────────────────────────────────────────────────────────

let _port = $state(7744);
let _config = $state<Record<string, string>>({});
let _loaded = $state(false);

// ── Getters ──────────────────────────────────────────────────────────────────

export function getPort(): number { return _port; }
export function isAppLoaded(): boolean { return _loaded; }

export function getConfigValue(key: string, fallback = ''): string {
  return _config[key] ?? fallback;
}

export function getActiveProjectId(): string | null {
  return _config['active_project'] || _config['active_solution'] || null;
}
/** @deprecated Use getActiveProjectId */
export function getActiveSolutionId(): string | null { return getActiveProjectId(); }

export function getSidebarMaxItems(): number {
  return parseInt(_config['sidebar_max_items'] ?? '5', 10);
}

export function getGlobalSkills(): string[] {
  try {
    return JSON.parse(_config['global_skills'] ?? '["zero-errors-policy","managing-project-sessions","pattern-based-development"]');
  } catch { return []; }
}

export function isSetupComplete(): boolean {
  return _config['setup_complete'] === '1';
}

export function getDismissedSuggestions(): string[] {
  try {
    return JSON.parse(_config['dismissed_suggestions'] ?? '[]');
  } catch { return []; }
}

// ── Setters ──────────────────────────────────────────────────────────────────

export async function setPort(port: number) {
  _port = port;
  // Port stays in localStorage — it's needed to find the daemon
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem('sensei:port', String(port));
  }
}

export async function setConfigValue(key: string, value: string) {
  _config[key] = value;
  _config = { ..._config };
  const api = senseiApi(_port);
  await api.setConfig({ [key]: value });
}

export async function setActiveProjectId(id: string | null) {
  if (id) {
    await setConfigValue('active_project', id);
  } else {
    delete _config['active_project'];
    _config = { ..._config };
    const api = senseiApi(_port);
    await api.deleteConfig('active_project');
  }
}
/** @deprecated Use setActiveProjectId */
export async function setActiveSolutionId(id: string | null) { return setActiveProjectId(id); }

export async function setSidebarMaxItems(val: number) {
  await setConfigValue('sidebar_max_items', String(Math.max(1, Math.min(20, val))));
}

export async function setGlobalSkills(skills: string[]) {
  await setConfigValue('global_skills', JSON.stringify(skills));
}

export async function setSetupComplete() {
  await setConfigValue('setup_complete', '1');
}

export async function dismissSuggestion(id: string) {
  const current = getDismissedSuggestions();
  if (!current.includes(id)) {
    await setConfigValue('dismissed_suggestions', JSON.stringify([...current, id]));
  }
}

// ── Lifecycle ────────────────────────────────────────────────────────────────

/** Load config from daemon. Call once on app startup. */
export async function loadAppState() {
  // Read port from localStorage (only thing that stays local)
  if (typeof localStorage !== 'undefined') {
    const stored = parseInt(localStorage.getItem('sensei:port') ?? '', 10);
    if (!isNaN(stored) && stored > 0) _port = stored;
  }

  // Fetch all config from daemon
  const api = senseiApi(_port);
  try {
    _config = await api.getConfig();
  } catch {
    // Daemon not running — use defaults
    _config = {};
  }

  _loaded = true;
}

/** Reset all config (for workspace reset). Clears everything on daemon + local. */
export async function resetAppState() {
  const api = senseiApi(_port);

  // Single daemon call clears everything: projects, solutions, config, graph, manifests
  try {
    await fetch(`http://127.0.0.1:${_port}/api/reset`, { method: 'POST' });
  } catch { /* non-fatal */ }

  _config = {};
  _loaded = false;

  // Clear all localStorage (preserve port)
  if (typeof localStorage !== 'undefined') {
    const port = localStorage.getItem('sensei:port');
    localStorage.clear();
    if (port) localStorage.setItem('sensei:port', port);
  }
}
