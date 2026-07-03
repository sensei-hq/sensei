// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import FtrStripHarness from './FtrStrip.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const points = (rates: number[]) => rates.map((rate) => ({ rate }));

describe('FtrStrip', () => {
  it('renders nothing when data is empty', () => {
    const m = mountComponent(FtrStripHarness, { data: [] });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="ftr-strip"]')).toBeNull();
  });

  it('renders one background + one foreground bar per data point', () => {
    const m = mountComponent(FtrStripHarness, { data: points([0.2, 0.5, 0.8, 1]) });
    cleanup.push(m.destroy);
    const svg = m.container.querySelector('[data-component="ftr-strip"]');
    expect(svg).toBeTruthy();
    // 4 background rects + 4 foreground rects = 8 total.
    expect(svg!.querySelectorAll('rect').length).toBe(8);
  });

  it('marks the most-recent bar with the today testid', () => {
    const m = mountComponent(FtrStripHarness, { data: points([0.3, 0.6, 0.9]) });
    cleanup.push(m.destroy);
    const today = m.container.querySelectorAll('[data-testid="ftr-bar-today"]');
    expect(today.length).toBe(1);
  });

  it('bar heights clamp to the [3, height] range', () => {
    // 0 → clamped up to 3px minimum; 1 → clamped down to full height.
    const m = mountComponent(FtrStripHarness, {
      data: points([0, 1]), height: 40, width: 80,
    });
    cleanup.push(m.destroy);
    const rects = m.container.querySelectorAll('[data-component="ftr-strip"] rect');
    // rects are [bg0, fg0, bg1, fg1]. fg0 (rate 0) height must be 3; fg1 (rate 1) must be 40.
    expect(rects[1].getAttribute('height')).toBe('3');
    expect(rects[3].getAttribute('height')).toBe('40');
  });

  it('propagates dim state onto the data attribute so callers can query mode', () => {
    const m = mountComponent(FtrStripHarness, { data: points([0.5]), dim: true });
    cleanup.push(m.destroy);
    const svg = m.container.querySelector('[data-component="ftr-strip"]') as SVGElement;
    expect(svg.getAttribute('data-dim')).toBe('true');
  });

  it('omits data-dim when dim is false so no stray attribute lands', () => {
    const m = mountComponent(FtrStripHarness, { data: points([0.5]) });
    cleanup.push(m.destroy);
    const svg = m.container.querySelector('[data-component="ftr-strip"]') as SVGElement;
    expect(svg.hasAttribute('data-dim')).toBe(false);
  });

  it('renders "14d ago" and "today" tick labels', () => {
    const m = mountComponent(FtrStripHarness, { data: points([0.5]) });
    cleanup.push(m.destroy);
    const text = m.container.querySelector('[data-component="ftr-strip"]')!.textContent ?? '';
    expect(text).toContain('14d ago');
    expect(text).toContain('today');
  });
});
