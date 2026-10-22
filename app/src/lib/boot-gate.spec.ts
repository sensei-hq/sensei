import { describe, it, expect, beforeEach } from 'vitest';
import { gateTarget } from './boot-gate.js';

// gateTarget is pure: the gate's inputs are passed in, so every case states the
// state it is about rather than mutating shared singletons.
let state = { healthOk: false, setupOk: true };
function setHealth(status: 'ok' | 'checking' | 'resolving' | 'needs-action') {
  state = { ...state, healthOk: status === 'ok' };
}
function setSetupComplete(complete: boolean) {
  state = { ...state, setupOk: complete };
}
function target(href: string): string | undefined {
  return gateTarget(new URL(href).pathname, state);
}

describe('gateTarget', () => {
  beforeEach(() => {
    setHealth('checking');
    setSetupComplete(true); // default for health-tier tests
  });

  describe('health tier', () => {
    it('returns undefined when already on /health (no redirect loop)', () => {
      setHealth('needs-action');
      expect(target('http://localhost/health')).toBeUndefined();
    });

    it('lets /logs and /upgrade through during health resolution', () => {
      setHealth('needs-action');
      expect(target('http://localhost/logs')).toBeUndefined();
      expect(target('http://localhost/upgrade')).toBeUndefined();
    });

    it('redirects to /health when status is not ok and path is gated', () => {
      setHealth('needs-action');
      expect(target('http://localhost/')).toBe('/health');
      expect(target('http://localhost/setup/welcome')).toBe('/health');
      expect(target('http://localhost/some/deep/route')).toBe('/health');
    });

    it('redirects to /health while still checking (initial load)', () => {
      setHealth('checking');
      expect(target('http://localhost/')).toBe('/health');
    });

    it('redirects to /health while actively resolving', () => {
      setHealth('resolving');
      expect(target('http://localhost/')).toBe('/health');
    });

    it('ignores query strings and only inspects pathname', () => {
      setHealth('needs-action');
      expect(target('http://localhost/health?foo=bar')).toBeUndefined();
      expect(target('http://localhost/other?foo=bar')).toBe('/health');
    });
  });

  describe('setup tier (once health is ok)', () => {
    beforeEach(() => setHealth('ok'));

    it('redirects to /setup/welcome when setup is incomplete', () => {
      setSetupComplete(false);
      expect(target('http://localhost/')).toBe('/setup/welcome');
      expect(target('http://localhost/workspaces')).toBe('/setup/welcome');
      expect(target('http://localhost/settings')).toBe('/setup/welcome');
    });

    it('allows /setup/* through even when incomplete', () => {
      setSetupComplete(false);
      expect(target('http://localhost/setup/welcome')).toBeUndefined();
      expect(target('http://localhost/setup/scan')).toBeUndefined();
      expect(target('http://localhost/setup/workspaces')).toBeUndefined();
    });

    it('allows /health, /logs, /upgrade through even when incomplete', () => {
      setSetupComplete(false);
      expect(target('http://localhost/health')).toBeUndefined();
      expect(target('http://localhost/logs')).toBeUndefined();
      expect(target('http://localhost/upgrade')).toBeUndefined();
    });

    it('does not redirect when setup is complete — any path is allowed', () => {
      setSetupComplete(true);
      expect(target('http://localhost/')).toBeUndefined();
      expect(target('http://localhost/workspaces')).toBeUndefined();
      expect(target('http://localhost/setup/welcome')).toBeUndefined();
    });
  });

  // A workspace window is opened deliberately from an already-healthy main
  // window, but it boots with its own fresh JS heap where healthState starts
  // 'checking' (isOk=false). Without a bypass, the gate bounces the fresh
  // window to /health and the workspace never renders — the "workspace window
  // won't open" bug. Workspace-window routes (`/workspace/{id}/...`) must pass the
  // gates unconditionally.
  describe('workspace window (fresh heap, gates not yet satisfied)', () => {
    it('lets /workspace/{id}/* through while health is still checking', () => {
      setHealth('checking');
      expect(target('http://localhost/workspace/abc-123/overview')).toBeUndefined();
    });

    it('lets /workspace/{id}/* through while health needs action', () => {
      setHealth('needs-action');
      expect(target('http://localhost/workspace/abc-123/sessions')).toBeUndefined();
    });

    it('lets /workspace/{id}/* through even when setup is incomplete', () => {
      setHealth('ok');
      setSetupComplete(false);
      expect(target('http://localhost/workspace/abc-123/about')).toBeUndefined();
    });

    it('still gates the observatory /workspaces list (plural) when not ok', () => {
      setHealth('needs-action');
      expect(target('http://localhost/workspaces')).toBe('/health');
    });
  });
});
