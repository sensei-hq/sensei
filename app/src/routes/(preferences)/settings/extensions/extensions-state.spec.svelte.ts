import { describe, it, expect, vi } from 'vitest';
import { ExtensionsState, type ExtensionsApi } from './extensions-state.svelte';

const ok = <T>(data: T) => ({ ok: true as const, data });
const fail = (status: number, message: string) => ({ ok: false as const, error: { status, message } });
const ext = (name: string, enabled = true) => ({ name, kind: 'skill', enabled });

function api(over: Partial<ExtensionsApi> = {}): ExtensionsApi {
  return { setInstalledItemEnabled: vi.fn().mockResolvedValue(ok({})), ...over } as ExtensionsApi;
}

describe('ExtensionsState', () => {
  it('turns an extension off once the daemon confirms', async () => {
    const s = new ExtensionsState({ extensions: [ext('a'), ext('b')], error: null });
    const a = api();
    await s.toggle(a, s.extensions[0], false);
    expect(a.setInstalledItemEnabled).toHaveBeenCalledWith('a', 'skill', false);
    expect(s.extensions.map((e) => e.enabled)).toEqual([false, true]);
    expect(s.isBusy(s.extensions[0])).toBe(false);
    expect(s.toggleError(s.extensions[0])).toBeNull();
  });

  it('leaves a refused toggle as it was and says why', async () => {
    const s = new ExtensionsState({ extensions: [ext('a')], error: null });
    await s.toggle(api({ setInstalledItemEnabled: vi.fn().mockResolvedValue(fail(409, 'Conflict')) }), s.extensions[0], false);
    expect(s.extensions[0].enabled).toBe(true);
    expect(s.toggleError(s.extensions[0])).toContain('Conflict');
    expect(s.isBusy(s.extensions[0])).toBe(false);
  });

  it('a later success clears the row’s error', async () => {
    const s = new ExtensionsState({ extensions: [ext('a')], error: null });
    await s.toggle(api({ setInstalledItemEnabled: vi.fn().mockResolvedValue(fail(500, 'boom')) }), s.extensions[0], false);
    await s.toggle(api(), s.extensions[0], false);
    expect(s.toggleError(s.extensions[0])).toBeNull();
  });

  it('keeps why the list could not be read; a re-read replaces it', () => {
    const s = new ExtensionsState({ extensions: [], error: 'Service Unavailable' });
    expect(s.loadError).toBe('Service Unavailable');
    s.setRead({ extensions: [ext('a')], error: null });
    expect(s.loadError).toBeNull();
    expect(s.extensions).toHaveLength(1);
  });
});

describe('toggleNotice — through the shared Notice', () => {
  it('names the extension and what it was switching to; the reason is the detail', async () => {
    const s = new ExtensionsState({ extensions: [ext('tdd')], error: null });
    const e = s.extensions[0];
    await s.toggle({ setInstalledItemEnabled: async () => ({ ok: false as const, error: { status: 409, message: 'Conflict' } }) } as never, e, !e.enabled);
    expect(s.toggleNotice(e)).toMatchObject({ tone: 'fault', detail: expect.stringContaining('Conflict') });
    expect(s.toggleNotice(e)!.title).toMatch(/^Couldn't switch tdd (on|off)$/);
  });
});
