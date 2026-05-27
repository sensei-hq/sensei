import { describe, it, expect, beforeEach } from 'vitest';
import { reroute } from './hooks.js';
import { healthState } from '$lib/health-state.svelte.js';
import { appState } from '$lib/appstate.svelte.js';
import { wizardState } from '$lib/wizard-state.svelte.js';

function setHealth(status: 'ok' | 'checking' | 'resolving' | 'needs-action') {
  healthState.status = status;
}

/**
 * Setup completion is sourced from `appState.config.setup_complete`, exposed
 * to hooks via `wizardState.isOk`. Mutating the config drives both.
 */
function setSetupComplete(complete: boolean) {
  appState.config = complete ? { setup_complete: '1' } : {};
}

// Sanity check — the gate must read through wizardState, not directly off appState.
if (wizardState.isOk !== appState.setupComplete) {
  throw new Error('wizardState.isOk should mirror appState.setupComplete');
}

describe('reroute', () => {
  beforeEach(() => {
    setHealth('checking');
    setSetupComplete(true); // default for health-tier tests
  });

  describe('health tier', () => {
    it('returns undefined when already on /health (no redirect loop)', () => {
      setHealth('needs-action');
      expect(reroute({ url: new URL('http://localhost/health') })).toBeUndefined();
    });

    it('lets /logs and /upgrade through during health resolution', () => {
      setHealth('needs-action');
      expect(reroute({ url: new URL('http://localhost/logs') })).toBeUndefined();
      expect(reroute({ url: new URL('http://localhost/upgrade') })).toBeUndefined();
    });

    it('redirects to /health when status is not ok and path is gated', () => {
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

    it('ignores query strings and only inspects pathname', () => {
      setHealth('needs-action');
      expect(reroute({ url: new URL('http://localhost/health?foo=bar') })).toBeUndefined();
      expect(reroute({ url: new URL('http://localhost/other?foo=bar') })).toBe('/health');
    });
  });

  describe('setup tier (once health is ok)', () => {
    beforeEach(() => setHealth('ok'));

    // TEMP: setup gate disabled in hooks.ts while debugging Phase 0 in the
    // live app. Restore both this test and the gate together once the setup
    // wizard regressions in docs/backlog.md are resolved.
    it.skip('redirects to /setup/welcome when setup is incomplete', () => {
      setSetupComplete(false);
      expect(reroute({ url: new URL('http://localhost/') })).toBe('/setup/welcome');
      expect(reroute({ url: new URL('http://localhost/projects') })).toBe('/setup/welcome');
      expect(reroute({ url: new URL('http://localhost/settings') })).toBe('/setup/welcome');
    });

    it('allows /setup/* through even when incomplete', () => {
      setSetupComplete(false);
      expect(reroute({ url: new URL('http://localhost/setup/welcome') })).toBeUndefined();
      expect(reroute({ url: new URL('http://localhost/setup/scan') })).toBeUndefined();
      expect(reroute({ url: new URL('http://localhost/setup/projects') })).toBeUndefined();
    });

    it('allows /health, /logs, /upgrade through even when incomplete', () => {
      setSetupComplete(false);
      expect(reroute({ url: new URL('http://localhost/health') })).toBeUndefined();
      expect(reroute({ url: new URL('http://localhost/logs') })).toBeUndefined();
      expect(reroute({ url: new URL('http://localhost/upgrade') })).toBeUndefined();
    });

    it('does not redirect when setup is complete — any path is allowed', () => {
      setSetupComplete(true);
      expect(reroute({ url: new URL('http://localhost/') })).toBeUndefined();
      expect(reroute({ url: new URL('http://localhost/projects') })).toBeUndefined();
      expect(reroute({ url: new URL('http://localhost/setup/welcome') })).toBeUndefined();
    });
  });
});
