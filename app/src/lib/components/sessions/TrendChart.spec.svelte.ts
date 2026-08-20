// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Harness from './TrendChart.harness.svelte';
import type { DayBucket } from '$lib/sessions-digest.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// Minimal bucket factory — only the fields the chart reads.
function bucket(over: Partial<DayBucket> = {}): DayBucket {
  return {
    day: '2026-07-01',
    label: '−7',
    total: 0,
    good: 0,
    bad: 0,
    ugly: 0,
    neutral: 0,
    mins: 0,
    goodMins: 0,
    badMins: 0,
    uglyMins: 0,
    chartedMins: 0,
    ftr: null,
    tokensIn: 0,
    tokensOut: 0,
    tokens: 0,
    ...over,
  };
}

function mount(buckets: DayBucket[]) {
  const m = mountComponent(Harness, { buckets });
  cleanup.push(m.destroy);
  return m;
}

const svg = (m: { container: HTMLElement }) =>
  m.container.querySelector('[data-component="trend-chart"]') as SVGElement;

describe('TrendChart', () => {
  it('renders the chart svg', () => {
    const m = mount([bucket({ day: 'd1', ftr: 1, total: 1, good: 1 })]);
    expect(svg(m)).toBeTruthy();
  });

  it('draws one dot per scored day and skips empty days', () => {
    const buckets = [
      bucket({ day: 'd1', ftr: 1, total: 2, good: 2 }),
      bucket({ day: 'd2', ftr: null }), // empty — no dot
      bucket({ day: 'd3', ftr: 0.5, total: 2, good: 1, bad: 1 }),
    ];
    const m = mount(buckets);
    expect(svg(m).querySelectorAll('circle').length).toBe(2);
    expect(svg(m).getAttribute('data-empty')).toBeNull();
  });

  it('colours a day dot by its dominant quality', () => {
    // Any abandoned session dominates the day → accent dot.
    const m = mount([bucket({ day: 'd1', ftr: 0, total: 1, ugly: 1 })]);
    const dot = svg(m).querySelector('circle');
    expect(dot?.getAttribute('fill')).toBe('var(--accent)');
  });

  it('marks itself empty and shows a quiet message when nothing is scored', () => {
    const m = mount([bucket({ day: 'd1', ftr: null }), bucket({ day: 'd2', ftr: null })]);
    expect(svg(m).getAttribute('data-empty')).toBe('true');
    expect(svg(m).querySelectorAll('circle').length).toBe(0);
    expect(svg(m).textContent ?? '').toContain('no scored sessions');
  });
});
