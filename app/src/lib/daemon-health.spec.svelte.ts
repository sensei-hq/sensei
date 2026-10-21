// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from 'vitest';
import { DaemonHealth } from './daemon-health.svelte.js';

describe('DaemonHealth', () => {
  it('defaults to unknown mode (no banner)', () => {
    const d = new DaemonHealth();
    expect(d.dbMode).toBeUndefined();
    expect(d.isDegraded).toBe(false);
  });

  it('apply() reflects a degraded daemon payload', () => {
    const d = new DaemonHealth();
    d.apply({ daemonDbMode: 'degraded' });
    expect(d.dbMode).toBe('degraded');
    expect(d.isDegraded).toBe(true);
  });

  // A first install has no database yet. That is a WAIT, not a fault, so it
  // must NOT raise the degraded banner — which says something has broken and
  // offers recovery. It gets its own flag so a surface can say "setting up".
  it('apply() treats provisioning as setting-up, never as degraded', () => {
    const d = new DaemonHealth();
    d.apply({ daemonDbMode: 'provisioning' });
    expect(d.dbMode).toBe('provisioning');
    expect(d.isProvisioning).toBe(true);
    expect(d.isDegraded).toBe(false);
  });

  it('apply() clears provisioning once the daemon reports full', () => {
    const d = new DaemonHealth();
    d.apply({ daemonDbMode: 'provisioning' });
    d.apply({ daemonDbMode: 'full' });
    expect(d.isProvisioning).toBe(false);
    expect(d.isDegraded).toBe(false);
  });

  it('apply() clears degraded when the daemon reports full', () => {
    const d = new DaemonHealth();
    d.apply({ daemonDbMode: 'degraded' });
    d.apply({ daemonDbMode: 'full' });
    expect(d.isDegraded).toBe(false);
  });

  it('apply() treats a payload without the field as unknown (no banner)', () => {
    const d = new DaemonHealth();
    d.apply({ daemonDbMode: 'degraded' });
    d.apply({});
    expect(d.dbMode).toBeUndefined();
    expect(d.isDegraded).toBe(false);
  });
});

// #288: a poll reads the daemon's mode, or — when /health cannot be read —
// leaves it unknown (no banner) rather than reading a mode from a default.
describe('DaemonHealth.poll', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('takes the mode the daemon reports', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ daemonDbMode: 'degraded' }), { status: 200 })));
    const h = new DaemonHealth();
    await h.poll();
    expect(h.dbMode).toBe('degraded');
  });

  it('is unknown when /health cannot be read, even after a known mode', async () => {
    const h = new DaemonHealth();
    h.apply({ daemonDbMode: 'degraded' });
    vi.stubGlobal('fetch', vi.fn(async () => new Response('boom', { status: 503 })));
    await h.poll();
    expect(h.dbMode).toBeUndefined();
  });
});
