// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import InsightCardHarness from './InsightCard.harness.svelte';
import type { ObservatoryInsight } from '$lib/types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const insight = (over: Partial<ObservatoryInsight> = {}): ObservatoryInsight => ({
  kanji: '繰',
  label: 'Pattern recurring',
  text: 'Cache invalidation missed again in session s-2891.',
  tag: '3rd time',
  tone: 'warn',
  ...over,
});

describe('InsightCard', () => {
  it('renders the kanji, label, text and tag', () => {
    const m = mountComponent(InsightCardHarness, { insight: insight() });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('繰');
    expect(m.container.textContent).toContain('Pattern recurring');
    expect(m.container.textContent).toContain('Cache invalidation missed again');
    expect(m.container.textContent).toContain('3rd time');
  });

  it('links to the insights triage surface by default', () => {
    const m = mountComponent(InsightCardHarness, { insight: insight() });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="insight-card"]')?.getAttribute('href')).toBe('/insights');
  });

  it('exposes the tone as a data attribute', () => {
    const m = mountComponent(InsightCardHarness, { insight: insight({ tone: 'good' }) });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="insight-card"]')?.getAttribute('data-tone')).toBe('good');
  });

  it('paints a warn insight with warning tokens', () => {
    const m = mountComponent(InsightCardHarness, { insight: insight({ tone: 'warn' }) });
    cleanup.push(m.destroy);
    const glyph = m.container.querySelector('.kanji');
    expect(glyph?.className).toContain('text-warning');
    const tag = m.container.querySelector('[data-insight-tag]');
    // Flipping paper surface + tone border/text (dark-safe), not a single-pole soft fill.
    expect(tag?.className).toContain('bg-paper-mute');
    expect(tag?.className).toContain('border-warning');
    expect(tag?.className).toContain('text-warning');
  });

  it('paints a good insight with success tokens', () => {
    const m = mountComponent(InsightCardHarness, { insight: insight({ tone: 'good' }) });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('.kanji')?.className).toContain('text-success');
    expect(m.container.querySelector('[data-insight-tag]')?.className).toContain('bg-paper-mute');
    expect(m.container.querySelector('[data-insight-tag]')?.className).toContain('border-success');
  });

  it('paints a mute insight with neutral ink tokens', () => {
    const m = mountComponent(InsightCardHarness, { insight: insight({ tone: 'mute' }) });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('.kanji')?.className).toContain('text-ink-mute');
    expect(m.container.querySelector('[data-insight-tag]')?.className).toContain('bg-paper-mute');
  });
});
