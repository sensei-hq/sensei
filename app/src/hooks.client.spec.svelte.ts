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

describe('hooks.client.ts — upgrade gate (L2)', () => {
  beforeEach(() => {
    sessionStore.clear();
    localStore.clear();
    sessionStorage.setItem('sensei:health', 'ready');
    localStorage.setItem('sensei:setup-complete', '1');
  });

  it('redirects "/" to /upgrade when sensei:app-version is set', () => {
    localStorage.setItem('sensei:app-version', '0.2.13');
    expect(reroute({ url: new URL('http://localhost/') })).toBe('/upgrade');
  });

  it('redirects /observatory to /upgrade when sensei:app-version is set', () => {
    localStorage.setItem('sensei:app-version', '0.2.13');
    expect(reroute({ url: new URL('http://localhost/observatory') })).toBe('/upgrade');
  });

  it('does not redirect /upgrade itself (exempt)', () => {
    localStorage.setItem('sensei:app-version', '0.2.13');
    expect(reroute({ url: new URL('http://localhost/upgrade') })).toBeUndefined();
  });

  it('does not redirect when no upgrade is pending', () => {
    expect(reroute({ url: new URL('http://localhost/') })).toBeUndefined();
  });

  it('does not redirect when stored version equals running app version (L10)', () => {
    // hooks.client.ts compares the stored value against the build-time-injected
    // __SENSEI_APP_VERSION__. In tests that constant is undefined and falls back
    // to "", so a stored "" should be treated as matching and skip the redirect.
    localStorage.setItem('sensei:app-version', '');
    expect(reroute({ url: new URL('http://localhost/') })).toBeUndefined();
  });
});
