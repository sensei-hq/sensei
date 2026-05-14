// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';

const sessionStore = new Map<string, string>();
const localStore = new Map<string, string>();

vi.stubGlobal('sessionStorage', {
  getItem: (k: string) => sessionStore.get(k) ?? null,
  setItem: (k: string, v: string) => sessionStore.set(k, v),
  removeItem: (k: string) => sessionStore.delete(k),
});
vi.stubGlobal('localStorage', {
  getItem: (k: string) => localStore.get(k) ?? null,
  setItem: (k: string, v: string) => localStore.set(k, v),
  removeItem: (k: string) => localStore.delete(k),
});

import { HealthState } from '$lib/health-state.svelte.js';
import { COMPONENT_ORDER } from '$lib/health-types.js';
import type { HealthPayload } from '$lib/health-types.js';
import { reroute } from './hooks.client.js';

const ok = (): HealthPayload => ({
  version: '0.2.14',
  uptimeSeconds: 0,
  platform: 'macos',
  packageManager: { id: 'homebrew', label: 'Homebrew', note: null, status: 'ready', version: '4.2.0', detail: null },
  components: COMPONENT_ORDER.map((id) => ({
    id, label: id, note: null, status: 'ready' as const, version: '1.0', detail: null,
  })),
  status: 'ok',
  remedy: null,
});

describe('hooks.client.ts ↔ HealthState wire compat', () => {
  beforeEach(() => {
    sessionStore.clear();
    localStore.clear();
    localStorage.setItem('sensei:setup-complete', '1');
  });

  it('HealthState.apply(ok) writes the same key/value that reroute reads', () => {
    const s = new HealthState();
    s.apply(ok());
    expect(sessionStorage.getItem('sensei:health')).toBe('ready');
    expect(reroute({ url: new URL('http://localhost/observatory') })).toBeUndefined();
  });
});
