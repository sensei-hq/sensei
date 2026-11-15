// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Harness from './TrendChart.harness.svelte';
import type { TurnDay } from '$lib/sessions-digest.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// Minimal day factory — only the fields the chart reads.
function day(over: Partial<TurnDay> = {}): TurnDay {
  return {
    day: '2026-07-01',
    label: '07/01',
    turns: 0,
    corrections: 0,
    ftr: null,
    tone: 'neutral',
    tokensIn: null,
    tokensOut: null,
    tokens: null,
    ...over,
  };
}

function mount(days: TurnDay[]) {
  const m = mountComponent(Harness, { days });
  cleanup.push(m.destroy);
  return m;
}

const svg = (m: { container: HTMLElement }) =>
  m.container.querySelector('[data-component="trend-chart"]') as SVGElement;

describe('TrendChart', () => {
  it('renders the chart svg', () => {
    const m = mount([day({ day: 'd1', ftr: 1, turns: 1, tone: 'good' })]);
    expect(svg(m)).toBeTruthy();
  });

  it('draws one dot per day with turns and skips empty days', () => {
    const m = mount([
      day({ day: 'd1', ftr: 1, turns: 2, tone: 'good' }),
      day({ day: 'd2', ftr: null }), // no turns — no dot
      day({ day: 'd3', ftr: 0.5, turns: 2, corrections: 1, tone: 'bad' }),
    ]);
    expect(svg(m).querySelectorAll('circle').length).toBe(2);
    expect(svg(m).getAttribute('data-empty')).toBeNull();
  });

  it('colours a day with corrected turns apart from an all-first-try one', () => {
    const m = mount([day({ day: 'd1', ftr: 0.5, turns: 2, corrections: 1, tone: 'bad' })]);
    const dot = svg(m).querySelector('circle');
    expect(dot?.getAttribute('fill')).toBe('var(--warning)');
  });

  it('marks itself empty and shows a quiet message when nothing is scored', () => {
    const m = mount([day({ day: 'd1', ftr: null }), day({ day: 'd2', ftr: null })]);
    expect(svg(m).getAttribute('data-empty')).toBe('true');
    expect(svg(m).querySelectorAll('circle').length).toBe(0);
    expect(svg(m).textContent ?? '').toContain('no turns in this range yet');
  });
});
