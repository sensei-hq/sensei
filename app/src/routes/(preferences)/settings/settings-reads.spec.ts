import { describe, it, expect } from 'vitest';
import { readAssistants, readExtensions, readPreferences } from './settings-reads';
import { DEFAULT_PREFERENCES } from './preferences-form';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };

// #288: each Settings page reads the daemon or says why it could not — a
// failed config read must not become a form of defaults (which the General
// page's auto-save would then write back over the user's settings).
describe('readPreferences', () => {
  it('reads the saved preferences', async () => {
    const saved = { ...DEFAULT_PREFERENCES, showWelcome: !DEFAULT_PREFERENCES.showWelcome };
    const d = await readPreferences({ tryGetConfig: async () => ok({ 'setup.preferences': JSON.stringify(saved) }) });
    expect([d.error, d.prefs?.showWelcome]).toEqual([null, saved.showWelcome]);
  });
  // D83 batch A, audit row 73: corrupt stored preferences read as the defaults,
  // and the page's auto-save wrote them over the real values.
  it('corrupt stored preferences are an error naming them, and no form (D83 batch A)', async () => {
    const d = await readPreferences({ tryGetConfig: async () => ok({ 'setup.preferences': '{not json' }) });
    expect(d.prefs).toBeNull();
    expect(d.error).toMatch(/stored preferences/i);
  });
  it('a failed read is an error and no form — never the defaults', async () => {
    expect(await readPreferences({ tryGetConfig: async () => down })).toEqual({ prefs: null, error: 'Service Unavailable', prefilled: false });
  });

  // The display name defaults to the machine's user name, as the setup wizard
  // already does — Settings showed it empty when the wizard never saved one.
  it('fills an empty display name from the system user name, and says so', async () => {
    const d = await readPreferences({ tryGetConfig: async () => ok({}) }, async () => 'Jerry');
    expect([d.prefs?.displayName, d.prefilled]).toEqual(['Jerry', true]);
  });
  it('never replaces a saved display name', async () => {
    const d = await readPreferences({ tryGetConfig: async () => ok({ user_name: 'J. Thomas' }) }, async () => 'Jerry');
    expect([d.prefs?.displayName, d.prefilled]).toEqual(['J. Thomas', false]);
  });
  it('leaves it empty when the system offers no name', async () => {
    const d = await readPreferences({ tryGetConfig: async () => ok({}) }, async () => '');
    expect([d.prefs?.displayName, d.prefilled]).toEqual(['', false]);
  });
  it('does not guess on a failed read', async () => {
    let asked = false;
    await readPreferences({ tryGetConfig: async () => down }, async () => ((asked = true), 'Jerry'));
    expect(asked).toBe(false);
  });
});

describe('readAssistants', () => {
  it('reads each detected assistant', async () => {
    const d = await readAssistants({ tryDetectAssistants: async () => ok([{ id: 'cc', family: 'claude', name: 'Claude Code', version: '2.1', installed: true, configured: true, config_path: '/x' }]) });
    expect(d).toEqual({ assistants: [{ family: 'claude', name: 'Claude Code', version: '2.1', configured: true }], error: null });
  });
  it('a failed read is an error, not "none detected"', async () => {
    expect(await readAssistants({ tryDetectAssistants: async () => down })).toEqual({ assistants: [], error: 'Service Unavailable' });
  });
});

describe('readExtensions', () => {
  it('reads each installed extension', async () => {
    const d = await readExtensions({ tryGetInstalledItems: async () => ok([{ name: 'tdd', kind: 'skill', path: '/x', enabled: false }]) });
    expect(d).toEqual({ extensions: [{ name: 'tdd', kind: 'skill', enabled: false }], error: null });
  });
  it('a failed read is an error, not "none installed"', async () => {
    expect(await readExtensions({ tryGetInstalledItems: async () => down })).toEqual({ extensions: [], error: 'Service Unavailable' });
  });

  // D83, L0 ruling on L5 settings/extensions tests r1: `enabled ?? true` draws
  // a row the daemon never reported as on, and the toggle would then write that
  // invented state back. Unknown is not a state to guess.
  it('a row with no `enabled` is unknown, never on', async () => {
    const d = await readExtensions({ tryGetInstalledItems: async () => ok([{ name: 'tdd', kind: 'skill', path: '/x' }]) });
    expect(d.extensions[0]?.name).toBe('tdd');
    expect(d.extensions[0]?.enabled ?? null).toBeNull();
  });

  // L0 ruling on tests r1 (L2): a row can be neither drawn nor toggled without
  // a kind and a name — the URL would say `…/unknown/enabled`. Skipped with the
  // reason, never invented.
  it('skips a row that reports no kind, with the reason', async () => {
    const d = await readExtensions({
      tryGetInstalledItems: async () => ok([{ name: 'ghost', path: '/x', enabled: true }, { name: 'tdd', kind: 'skill', path: '/y', enabled: true }]),
    });
    expect(d.extensions.map((e) => e.name)).toEqual(['tdd']);
    expect(d.skipped).toEqual([{ name: 'ghost', reason: 'kind not reported' }]);
  });

  it('skips a row that reports no name, with the reason', async () => {
    const d = await readExtensions({ tryGetInstalledItems: async () => ok([{ kind: 'skill', path: '/x', enabled: false }]) });
    expect(d.extensions).toEqual([]);
    expect(d.skipped).toEqual([{ name: '', reason: 'name not reported' }]);
  });

  // Review r1, N1: both missing is one reason, not "kind" alone.
  it('names both when the row reports neither', async () => {
    const d = await readExtensions({ tryGetInstalledItems: async () => ok([{ path: '/x', enabled: true }]) });
    expect(d.extensions).toEqual([]);
    expect(d.skipped).toEqual([{ name: '', reason: 'name and kind not reported' }]);
  });
});
