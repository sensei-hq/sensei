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
  it('a failed read is an error and no form — never the defaults', async () => {
    expect(await readPreferences({ tryGetConfig: async () => down })).toEqual({ prefs: null, error: 'Service Unavailable' });
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
});
