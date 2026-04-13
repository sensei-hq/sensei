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

export function getActiveSolutionId(): string | null {
  return _config['active_solution'] || null;
}

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

export async function setActiveSolutionId(id: string | null) {
  if (id) {
    await setConfigValue('active_solution', id);
  } else {
    delete _config['active_solution'];
    _config = { ..._config };
    const api = senseiApi(_port);
    await api.deleteConfig('active_solution');
  }
}

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

/** Reset all config (for workspace reset). */
export async function resetAppState() {
  const api = senseiApi(_port);
  // Delete all solutions on daemon
  try {
    const sols = await api.listSolutions();
    for (const s of sols) { await api.deleteSolution(s.id); }
  } catch { /* non-fatal */ }
  // Clear config
  const keys = Object.keys(_config);
  for (const key of keys) {
    await api.deleteConfig(key).catch(() => {});
  }
  _config = {};
  _loaded = false;
  // Clear localStorage remnants
  if (typeof localStorage !== 'undefined') {
    localStorage.removeItem('sensei:projects_raw');
    localStorage.removeItem('sensei:setup_complete');
    localStorage.removeItem('sensei:variant_overrides');
    localStorage.removeItem('sensei:index_states');
    localStorage.removeItem('sensei:solutions');
    localStorage.removeItem('sensei:active_solution');
    localStorage.removeItem('sensei:migration_v1');
    localStorage.removeItem('sensei:sidebar_max_items');
    localStorage.removeItem('sensei:global_skills');
    localStorage.removeItem('sensei:dismissed_suggestions');
  }
}
