// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
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
