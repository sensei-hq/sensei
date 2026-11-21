// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { scanState } from '$lib/scan.js';
import ScanProgress from './ScanProgress.svelte';

// The three variants read the one scan singleton: drive the state, assert
// the DOM. No attach() here — that would open sockets and timers — so the
// state is set directly and flushed.
let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  scanState.reset();
  scanState.expanded = false;
});

function scanning() {
  scanState.status = 'scanning';
  scanState.queueSnapshot = { pending: 80, running: 6, completed: 12 };
  flushSync();
}

function mount(variant: 'mini' | 'band' | 'full') {
  const m = mountComponent(ScanProgress, { variant });
  cleanup.push(m.destroy);
  return m;
}

describe('ScanProgress mini', () => {
  it('renders nothing while idle', () => {
    const m = mount('mini');
    expect(m.container.querySelector('[data-component="scan-mini"]')).toBeNull();
  });

  it('appears with live counts when a run starts', () => {
    const m = mount('mini');
    scanning();
    expect(m.container.querySelector('[data-component="scan-mini"]')).not.toBeNull();
    expect(m.container.textContent).toContain('Indexing · 86 queued');
  });

  it('expand opens the band', () => {
    const m = mount('mini');
    scanning();
    m.container.querySelector<HTMLButtonElement>('button[aria-label="Expand indexing progress"]')!.click();
    expect(scanState.expanded).toBe(true);
  });

  it('an unreachable daemon reads as retrying, not stale counts', () => {
    const m = mount('mini');
    scanning();
    scanState.daemonReachable = false;
    flushSync();
    expect(m.container.textContent).toContain('Daemon unreachable — retrying…');
    expect(m.container.textContent).not.toContain('Indexing · 86 queued');
  });

  it('a failed run says so through Notice', () => {
    const m = mount('mini');
    scanState.status = 'error';
    scanState.error = 'The scan did not start for /a: refused';
    flushSync();
    expect(m.container.textContent).toContain('Scan failed');
  });
});

describe('ScanProgress band', () => {
  it('renders nothing until expanded', () => {
    const m = mount('band');
    scanning();
    expect(m.container.querySelector('[data-component="scan-band"]')).toBeNull();
  });

  it('expanded while scanning shows stats, collapse hides it', () => {
    const m = mount('band');
    scanState.expanded = true;
    scanning();
    expect(m.container.querySelector('[data-component="scan-band"]')).not.toBeNull();
    expect(m.container.textContent?.replace(/\s+/g, ' ')).toContain(
      '0 roots · 0 discovered · 0 queued · 0 processed',
    );
    const buttons = [...m.container.querySelectorAll('button')].map((b) => b.textContent?.trim());
    expect(buttons).toContain('Full view');
    m.container.querySelectorAll('button').forEach((b) => {
      if (b.textContent?.trim() === 'Collapse') b.click();
    });
    expect(scanState.expanded).toBe(false);
    flushSync();
    expect(m.container.querySelector('[data-component="scan-band"]')).toBeNull();
  });

  it('hides itself when the run ends', () => {
    const m = mount('band');
    scanState.expanded = true;
    scanning();
    expect(m.container.querySelector('[data-component="scan-band"]')).not.toBeNull();
    scanState.status = 'idle';
    flushSync();
    expect(m.container.querySelector('[data-component="scan-band"]')).toBeNull();
  });
});

describe('ScanProgress full', () => {
  it('renders the scan view from the same state', () => {
    const m = mount('full');
    scanning();
    expect(m.container.querySelector('[data-testid="scan-tasks-panel"]')).not.toBeNull();
    expect(m.container.textContent).toContain('QUEUED');
  });

  it('before the run it offers Begin, which owns the run', () => {
    const m = mount('full');
    // Begin is offered but never clicked here: it would start a live run
    // against the daemon.
    expect(m.container.textContent).toContain('Begin scan →');
  });
});
