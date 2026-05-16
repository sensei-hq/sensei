import { describe, it, expect, beforeEach } from 'vitest';
import { reroute } from './hooks.ts';
import { healthState } from '$lib/health-state.svelte.ts';

/** Reset healthState between tests so each runs against a fresh status.
 *  The module exports a singleton, so we mutate the shared instance.
 *  `status` is a Svelte 5 $state — direct assignment is the supported
 *  mutation API. */
function setHealth(status: 'ok' | 'checking' | 'resolving' | 'needs-action') {
  healthState.status = status;
}

describe('reroute', () => {
  beforeEach(() => setHealth('checking'));

  it('returns undefined when already on /health (no redirect loop)', () => {
    setHealth('needs-action');
    const r = reroute({ url: new URL('http://localhost/health') });
    expect(r).toBeUndefined();
  });

  it('redirects to /health when status is not ok and path is not /health', () => {
    setHealth('needs-action');
    expect(reroute({ url: new URL('http://localhost/') })).toBe('/health');
    expect(reroute({ url: new URL('http://localhost/setup/welcome') })).toBe('/health');
    expect(reroute({ url: new URL('http://localhost/some/deep/route') })).toBe('/health');
  });

  it('redirects to /health while still checking (initial load)', () => {
    setHealth('checking');
    expect(reroute({ url: new URL('http://localhost/') })).toBe('/health');
  });

  it('redirects to /health while actively resolving', () => {
    setHealth('resolving');
    expect(reroute({ url: new URL('http://localhost/') })).toBe('/health');
  });

  it('does not redirect when health is ok — any path is allowed', () => {
    setHealth('ok');
    expect(reroute({ url: new URL('http://localhost/') })).toBeUndefined();
    expect(reroute({ url: new URL('http://localhost/setup/welcome') })).toBeUndefined();
    expect(reroute({ url: new URL('http://localhost/health') })).toBeUndefined();
  });

  it('ignores query strings and only inspects pathname', () => {
    setHealth('needs-action');
    expect(reroute({ url: new URL('http://localhost/health?foo=bar') })).toBeUndefined();
    expect(reroute({ url: new URL('http://localhost/other?foo=bar') })).toBe('/health');
  });
});
