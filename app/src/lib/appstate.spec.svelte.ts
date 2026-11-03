/**
 * Tests for AppState — reactive config, derived getters, state transitions.
 * Uses $state so this is .spec.svelte.ts.
 * API calls are mocked via vi.mock — we test state logic, not network.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { AppState } from './appstate.svelte.js';

// Mock senseiApi — prevent real HTTP calls. The mock surface is exposed as
// `apiMock` so individual tests can adjust per-call behavior.
const apiMock = {
  getConfig:    vi.fn().mockResolvedValue({ setup_complete: '1', active_workspace: 'proj-1' }),
  tryGetConfig: vi.fn().mockResolvedValue({ ok: true,  data: { setup_complete: '1', active_workspace: 'proj-1' } }),
  trySetConfig: vi.fn().mockResolvedValue({ ok: true,  data: undefined }),
  tryDeleteConfig: vi.fn().mockResolvedValue({ ok: true, data: undefined }),
  tryListWorkspaces: vi.fn().mockResolvedValue({ ok: true, data: [] }),
};

vi.mock('./api.js', async (importOriginal) => ({
  failureReason: (await importOriginal<typeof import('./api.js')>()).failureReason,
  senseiApi: () => apiMock,
}));

// Mock fetch for reset()
vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true }));

// Mock localStorage
const storage = new Map<string, string>();
vi.stubGlobal('localStorage', {
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, value: string) => storage.set(key, value),
  removeItem: (key: string) => storage.delete(key),
  clear: () => storage.clear(),
});

// Mock sessionStorage — HealthState owns 'sensei:health', AppState must not touch it
const sessionStore = new Map<string, string>();
vi.stubGlobal('sessionStorage', {
  getItem: (key: string) => sessionStore.get(key) ?? null,
  setItem: (key: string, value: string) => sessionStore.set(key, value),
  removeItem: (key: string) => sessionStore.delete(key),
  clear: () => sessionStore.clear(),
});

// Pretend Tauri is present so appState.load() takes the daemon-talking path.
const win = (globalThis as unknown as { window?: Record<string, unknown> }).window
  ?? ((globalThis as unknown as { window: Record<string, unknown> }).window = {});
win.__TAURI__ = {};

describe('AppState', () => {
  let state: AppState;

  beforeEach(() => {
    storage.clear();
    sessionStore.clear();
    apiMock.getConfig.mockResolvedValue({ setup_complete: '1', active_workspace: 'proj-1' });
    apiMock.tryGetConfig.mockResolvedValue({ ok: true, data: { setup_complete: '1', active_workspace: 'proj-1' } });
    apiMock.trySetConfig.mockResolvedValue({ ok: true, data: undefined });
    apiMock.tryDeleteConfig.mockResolvedValue({ ok: true, data: undefined });
    apiMock.tryListWorkspaces.mockResolvedValue({ ok: true, data: [] });
    state = new AppState();
  });

  // ── Defaults ───────────────────────────────────────────────

  it('starts with default port', () => {
    expect(state.port).toBe(7744);
  });

  it('starts with empty config', () => {
    expect(state.config).toEqual({});
  });

  it('starts not loaded', () => {
    expect(state.loaded).toBe(false);
  });

  // ── Derived getters ────────────────────────────────────────

  it('setupComplete is false when wizardState reports false', async () => {
    const { wizardState } = await import('./wizard-state.svelte.js');
    wizardState.setupComplete = false;
    expect(state.setupComplete).toBe(false);
  });

  it('setupComplete passes through wizardState.setupComplete (facade contract)', async () => {
    const { wizardState } = await import('./wizard-state.svelte.js');
    wizardState.setupComplete = true;
    expect(state.setupComplete).toBe(true);
    wizardState.setupComplete = false;
    expect(state.setupComplete).toBe(false);
  });

  it('activeWorkspaceId returns null when empty', () => {
    expect(state.activeWorkspaceId).toBeNull();
  });

  it('activeWorkspaceId reads from config', () => {
    state.config = { active_workspace: 'proj-42' };
    expect(state.activeWorkspaceId).toBe('proj-42');
  });

  it('activeWorkspaceId falls back to active_solution', () => {
    state.config = { active_solution: 'sol-1' };
    expect(state.activeWorkspaceId).toBe('sol-1');
  });

  it('sidebarMaxItems defaults to 5', () => {
    expect(state.sidebarMaxItems).toBe(5);
  });

  it('sidebarMaxItems reads from config', () => {
    state.config = { sidebar_max_items: '10' };
    expect(state.sidebarMaxItems).toBe(10);
  });

  it('globalSkills returns default list when empty', () => {
    const skills = state.globalSkills;
    expect(skills).toContain('zero-errors-policy');
    expect(skills.length).toBe(3);
  });

  it('globalSkills parses from config', () => {
    state.config = { global_skills: '["a","b"]' };
    expect(state.globalSkills).toEqual(['a', 'b']);
  });

  it('globalSkills returns empty on invalid JSON', () => {
    state.config = { global_skills: 'not-json' };
    expect(state.globalSkills).toEqual([]);
  });

  it('dismissedSuggestions defaults to empty', () => {
    expect(state.dismissedSuggestions).toEqual([]);
  });

  it('dismissedSuggestions parses from config', () => {
    state.config = { dismissed_suggestions: '["s1","s2"]' };
    expect(state.dismissedSuggestions).toEqual(['s1', 's2']);
  });

  // ── port is a readonly build-time constant ─────────────────

  it('port is a readonly build-time constant (no runtime override)', () => {
    // `setPort` and the localStorage `sensei:port` override are gone —
    // a stale value from a previous session was silently routing every
    // fetch to the wrong port. Port comes solely from DEFAULT_PORT now.
    expect(state.port).toBe(7744);
  });

  // ── setConfig ──────────────────────────────────────────────

  it('setConfig updates local config', async () => {
    await state.setConfig('foo', 'bar');
    expect(state.config['foo']).toBe('bar');
  });

  it('setConfig preserves existing keys', async () => {
    state.config = { existing: 'value' };
    await state.setConfig('new_key', 'new_value');
    expect(state.config['existing']).toBe('value');
    expect(state.config['new_key']).toBe('new_value');
  });

  // #304: a single-key write used to go through the legacy put, which only
  // warned — the cache kept a value the daemon never stored.
  it('setConfig that the daemon refuses keeps the old value and throws why', async () => {
    apiMock.trySetConfig.mockResolvedValueOnce({ ok: false, error: { status: 503, message: 'Service Unavailable' } });
    state.config = { sidebar_max_items: '5' };
    await expect(state.setSidebarMaxItems(8)).rejects.toThrow(/Service Unavailable/);
    expect(state.config['sidebar_max_items']).toBe('5');
  });

  // ── setActiveWorkspaceId ─────────────────────────────────────

  it('setActiveWorkspaceId sets workspace', async () => {
    await state.setActiveWorkspaceId('proj-1');
    expect(state.config['active_workspace']).toBe('proj-1');
  });

  it('setActiveWorkspaceId null clears workspace', async () => {
    state.config = { active_workspace: 'proj-1' };
    await state.setActiveWorkspaceId(null);
    expect(state.config['active_workspace']).toBeUndefined();
    expect(apiMock.tryDeleteConfig).toHaveBeenCalledWith('active_workspace');
  });

  // #304: the delete used to swallow every failure, so the cache said "no
  // active workspace" while the daemon still held one.
  it('a clear the daemon refuses keeps the active workspace and throws why', async () => {
    apiMock.tryDeleteConfig.mockResolvedValueOnce({ ok: false, error: { status: 0, message: 'connection refused' } });
    state.config = { active_workspace: 'proj-1' };
    await expect(state.setActiveWorkspaceId(null)).rejects.toThrow(/connection refused/);
    expect(state.config['active_workspace']).toBe('proj-1');
  });

  // ── setSidebarMaxItems ─────────────────────────────────────

  it('setSidebarMaxItems clamps to range', async () => {
    await state.setSidebarMaxItems(0);
    expect(state.config['sidebar_max_items']).toBe('1');

    await state.setSidebarMaxItems(50);
    expect(state.config['sidebar_max_items']).toBe('20');

    await state.setSidebarMaxItems(8);
    expect(state.config['sidebar_max_items']).toBe('8');
  });

  // ── setConfigs ─────────────────────────────────────────────
  // (setup_complete write moved to wizardState.setCompleted — tested there.
  //  These tests cover the batch-write + rollback semantics on appState.)

  it('setConfigs updates cache and calls daemon (via trySetConfig)', async () => {
    await state.setConfigs({ user_name: 'Dev', 'setup.preferences': '{}' });
    expect(state.config['user_name']).toBe('Dev');
    expect(state.config['setup.preferences']).toBe('{}');
    expect(apiMock.trySetConfig).toHaveBeenCalledWith({ user_name: 'Dev', 'setup.preferences': '{}' });
  });

  it('setConfigs rolls back cache when daemon write fails', async () => {
    apiMock.trySetConfig.mockResolvedValueOnce({ ok: false, error: { status: 500, message: 'boom' } });
    state.config = { user_name: 'Pre' };
    await expect(state.setConfigs({ user_name: 'Post' })).rejects.toThrow(/boom/);
    expect(state.config['user_name']).toBe('Pre');
  });

  // Settings is its own window (2026-10-27): a change made there must reach the
  // main window, which holds its own copy of the config.
  it('setConfigs tells the other windows what it saved — and nothing when the daemon refused', async () => {
    const told: string[][] = [];
    state.onConfigSaved = (keys) => told.push(keys);
    await state.setConfigs({ 'ui.glyphs': 'icon' });
    apiMock.trySetConfig.mockResolvedValueOnce({ ok: false, error: { status: 500, message: 'boom' } });
    await expect(state.setConfigs({ user_name: 'x' })).rejects.toThrow();
    expect(told).toEqual([['ui.glyphs']]);
    state.onConfigSaved = null;
  });

  it('setConfigs throws (not silent) on daemon error so callers can react', async () => {
    apiMock.trySetConfig.mockResolvedValueOnce({ ok: false, error: { status: 503, message: 'unreachable' } });
    await expect(state.setConfigs({ setup_complete: '1' })).rejects.toThrow();
  });

  // ── dismissSuggestion ──────────────────────────────────────

  it('dismissSuggestion adds to list', async () => {
    await state.dismissSuggestion('tip-1');
    const dismissed: string[] = JSON.parse(state.config['dismissed_suggestions']);
    expect(dismissed).toContain('tip-1');
  });

  it('dismissSuggestion does not duplicate', async () => {
    state.config = { dismissed_suggestions: '["tip-1"]' };
    await state.dismissSuggestion('tip-1');
    const dismissed: string[] = JSON.parse(state.config['dismissed_suggestions']);
    expect(dismissed.filter((d: string) => d === 'tip-1').length).toBe(1);
  });

  // ── load ───────────────────────────────────────────────────

  it('load sets loaded to true', async () => {
    await state.load();
    expect(state.loaded).toBe(true);
  });

  it('load populates config from API', async () => {
    await state.load();
    expect(state.config['setup_complete']).toBe('1');
    expect(state.config['active_workspace']).toBe('proj-1');
  });

  // The `setup_complete` ↔ localStorage reconciliation moved to
  // wizardState.hydrate() — that's the canonical owner of the
  // setupComplete flag. appState.load() no longer touches the
  // localStorage cache; it just hydrates `config`. Coverage for the
  // reconcile behaviour lives in wizard-state.spec.svelte.ts.

  it('load does not touch localStorage[setup-complete] (single-owner contract)', async () => {
    storage.set('sensei:setup-complete', 'sentinel');
    await state.load();
    expect(storage.get('sensei:setup-complete')).toBe('sentinel');
  });

  it('load leaves localStorage alone when daemon is unreachable (L4)', async () => {
    apiMock.tryGetConfig.mockResolvedValueOnce({ ok: false, error: { status: 0, message: 'Network error' } });
    storage.set('sensei:setup-complete', '1');
    await state.load();
    expect(storage.get('sensei:setup-complete')).toBe('1');
  });

  it('load resolves to void and marks state loaded on daemon success', async () => {
    await expect(state.load()).resolves.toBeUndefined();
    expect(state.loaded).toBe(true);
  });

  // A failed read is not an empty config (CLAUDE.md: never fabricate on a failure path).
  // A daemon restart (make install, brew services) refuses the read for a few seconds;
  // swapping in {} reset the glyph mode, the name and every preference in that window.
  it('a failed read keeps the last config, says why, and reads again shortly after', async () => {
    vi.useFakeTimers();
    try {
      state.config = { 'ui.glyphs': 'icon' };
      apiMock.tryGetConfig.mockResolvedValueOnce({ ok: false, error: { status: 0, message: 'Load failed' } });
      await state.load();
      expect(state.loaded).toBe(true);
      expect(state.config).toEqual({ 'ui.glyphs': 'icon' });
      expect(state.configError).toMatch(/Load failed/);
      apiMock.tryGetConfig.mockResolvedValueOnce({ ok: true, data: { 'ui.glyphs': 'kanji' } });
      await vi.advanceTimersByTimeAsync(2_000);
      expect(state.config).toEqual({ 'ui.glyphs': 'kanji' });
      expect(state.configError).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });

  // ── loadWorkspaceCount ───────────────────────────────────────

  it('workspaceCount starts null (no badge until loaded)', () => {
    expect(state.workspaceCount).toBeNull();
  });

  it('loadWorkspaceCount caches the number of workspaces from the daemon', async () => {
    apiMock.tryListWorkspaces.mockResolvedValueOnce({ ok: true, data: [{ id: 'a' }, { id: 'b' }, { id: 'c' }] });
    await state.loadWorkspaceCount();
    expect(state.workspaceCount).toBe(3);
  });

  // #288: a failed read used to leave the badge at 0 — "no workspaces".
  it('loadWorkspaceCount leaves the count unknown (null) when the read fails', async () => {
    apiMock.tryListWorkspaces.mockResolvedValueOnce({ ok: false, error: { status: 503, message: 'Service Unavailable' } });
    await state.loadWorkspaceCount();
    expect(state.workspaceCount).toBeNull();
  });

  it('loadWorkspaceCount is a no-op without Tauri (count stays null)', async () => {
    const tauri = win.__TAURI__;
    delete win.__TAURI__;
    await state.loadWorkspaceCount();
    expect(state.workspaceCount).toBeNull();
    win.__TAURI__ = tauri;
  });

  it('reset clears the cached workspace count', async () => {
    state.workspaceCount = 7;
    await state.reset();
    expect(state.workspaceCount).toBeNull();
  });

  // ── reset ──────────────────────────────────────────────────

  it('reset clears config and loaded', async () => {
    state.config = { foo: 'bar' };
    state.loaded = true;
    await state.reset();
    expect(state.config).toEqual({});
    expect(state.loaded).toBe(false);
  });

  it('reset does not touch sensei:health (HealthState owns the health cache)', async () => {
    sessionStore.clear();
    sessionStore.set('sensei:health', 'ready');
    state.config = { foo: 'bar' };
    state.loaded = true;
    await state.reset();
    expect(sessionStore.get('sensei:health')).toBe('ready');
  });

  it('reset preserves sensei:app-version so a staged upgrade is not skipped', async () => {
    // `sensei:port` is no longer a recognised key — port is a build-time
    // constant now, no longer read from or written to localStorage.
    storage.set('sensei:app-version', '0.2.13');
    storage.set('sensei:setup-complete', '1');
    storage.set('something-else', 'x');
    await state.reset();
    expect(storage.get('sensei:app-version')).toBe('0.2.13');  // must survive reset
    // non-protected keys can still be cleared
    expect(storage.has('sensei:setup-complete')).toBe(false);
    expect(storage.has('something-else')).toBe(false);
  });
});
