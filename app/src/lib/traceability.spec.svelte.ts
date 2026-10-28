import { describe, it, expect, vi } from 'vitest';
import { driftTone, driftChipClasses, driftDotStatus, DriftScan } from './traceability.svelte.js';

describe('driftTone', () => {
  it('maps broken → danger', () => expect(driftTone('broken')).toBe('danger'));
  it('maps drifted → warning', () => expect(driftTone('drifted')).toBe('warning'));
  it('degrades an unexpected status to ink (never dropped)', () => expect(driftTone('flapping')).toBe('ink'));
});

describe('driftChipClasses', () => {
  it('broken → danger tokens', () =>
    expect(driftChipClasses('broken')).toEqual({ bg: 'bg-danger-soft', text: 'text-on-danger-soft' }));
  it('drifted → warning tokens', () =>
    expect(driftChipClasses('drifted')).toEqual({ bg: 'bg-warning-soft', text: 'text-on-warning-soft' }));
});

// Audit item 10 (2026-10-28): the workspace screen drew an unknown drift status
// as a GREEN dot; the observatory drew it muted. One tone, both screens.
describe('driftDotStatus', () => {
  it('follows driftTone: broken fails, drifted warns, anything else is neutral — never ok', () => {
    expect(['broken', 'drifted', 'flapping', ''].map(driftDotStatus)).toEqual(['fail', 'warn', 'idle', 'idle']);
  });
});

describe('DriftScan', () => {
  // The workspace screen held the scan action and its flags in the component.
  it('runs a scan, keeps its summary, and re-reads the page', async () => {
    const reload = vi.fn(async () => {});
    const scan = new DriftScan(async () => ({ ok: true, data: { scannedDocs: 4, newBroken: 1, resolved: 0 } }) as never, reload);
    const run = scan.run('w1');
    expect(scan.scanning).toBe(true);
    expect(scan.buttonLabel).toBe('Scanning…');
    await run;
    expect(scan).toMatchObject({ scanning: false, summary: { scannedDocs: 4, newBroken: 1, resolved: 0 }, error: null });
    expect(scan.summaryText).toBe('Scanned 4 docs · added 1 broken · resolved 0.');
    expect(scan.buttonLabel).toBe('Scan now');
    expect(reload).toHaveBeenCalled();
  });

  it('says why a scan failed, and shows no summary', async () => {
    const scan = new DriftScan(async () => ({ ok: false, error: { message: 'daemon down' } }) as never, async () => {});
    await scan.run('w1');
    expect(scan.summary).toBeNull();
    expect(scan.error).toMatch(/daemon down/);
    expect(scan.errorText).toBe('The scan failed: daemon down');
    expect(scan.summaryText).toBeNull();
  });
});
