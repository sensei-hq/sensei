/**
 * Tests for AppState — reactive config, derived getters, state transitions.
 * Uses $state so this is .spec.svelte.ts.
 * API calls are mocked via vi.mock — we test state logic, not network.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { AppState } from './appstate.svelte.js';

// Mock senseiApi — prevent real HTTP calls
vi.mock('./api.js', () => ({
  senseiApi: () => ({
    getConfig: vi.fn().mockResolvedValue({ setup_complete: '1', active_project: 'proj-1' }),
    setConfig: vi.fn().mockResolvedValue(undefined),
    deleteConfig: vi.fn().mockResolvedValue(undefined),
  }),
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

describe('AppState', () => {
  let state: AppState;

  beforeEach(() => {
    storage.clear();
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

  it('setupComplete is false when config empty', () => {
    expect(state.setupComplete).toBe(false);
  });

  it('setupComplete is true when set', () => {
    state.config = { setup_complete: '1' };
    expect(state.setupComplete).toBe(true);
  });

  it('activeProjectId returns null when empty', () => {
    expect(state.activeProjectId).toBeNull();
  });

  it('activeProjectId reads from config', () => {
    state.config = { active_project: 'proj-42' };
    expect(state.activeProjectId).toBe('proj-42');
  });

  it('activeProjectId falls back to active_solution', () => {
    state.config = { active_solution: 'sol-1' };
    expect(state.activeProjectId).toBe('sol-1');
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

  // ── setPort ────────────────────────────────────────────────

  it('setPort updates port', async () => {
    await state.setPort(9999);
    expect(state.port).toBe(9999);
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

  // ── setActiveProjectId ─────────────────────────────────────

  it('setActiveProjectId sets project', async () => {
    await state.setActiveProjectId('proj-1');
    expect(state.config['active_project']).toBe('proj-1');
  });

  it('setActiveProjectId null clears project', async () => {
    state.config = { active_project: 'proj-1' };
    await state.setActiveProjectId(null);
    expect(state.config['active_project']).toBeUndefined();
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

  // ── setSetupComplete ───────────────────────────────────────

  it('setSetupComplete marks setup done', async () => {
    await state.setSetupComplete();
    expect(state.setupComplete).toBe(true);
  });

  // ── dismissSuggestion ──────────────────────────────────────

  it('dismissSuggestion adds to list', async () => {
    await state.dismissSuggestion('tip-1');
    const dismissed = JSON.parse(state.config['dismissed_suggestions']);
    expect(dismissed).toContain('tip-1');
  });

  it('dismissSuggestion does not duplicate', async () => {
    state.config = { dismissed_suggestions: '["tip-1"]' };
    await state.dismissSuggestion('tip-1');
    const dismissed = JSON.parse(state.config['dismissed_suggestions']);
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
    expect(state.config['active_project']).toBe('proj-1');
  });

  // ── reset ──────────────────────────────────────────────────

  it('reset clears config and loaded', async () => {
    state.config = { foo: 'bar' };
    state.loaded = true;
    await state.reset();
    expect(state.config).toEqual({});
    expect(state.loaded).toBe(false);
  });
});
