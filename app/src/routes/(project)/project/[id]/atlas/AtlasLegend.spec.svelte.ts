// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import AtlasLegendHarness from './AtlasLegend.harness.svelte';
import { legendItems } from './atlas-graph.svelte.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('AtlasLegend', () => {
  it('renders a swatch + label per legend item', () => {
    const m = mountComponent(AtlasLegendHarness, {
      items: legendItems(['function', 'file']),
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('function');
    expect(m.container.textContent).toContain('file');
    const swatches = m.container.querySelectorAll('[data-component="atlas-legend"] > span');
    expect(swatches).toHaveLength(2);
  });

  it('paints each swatch with the kind token colour', () => {
    const m = mountComponent(AtlasLegendHarness, { items: legendItems(['function']) });
    cleanup.push(m.destroy);
    const dot = m.container.querySelector('[data-component="atlas-legend"] span span') as HTMLElement;
    expect(dot.getAttribute('style')).toContain('var(--accent)');
  });

  it('renders nothing but the container when there are no items', () => {
    const m = mountComponent(AtlasLegendHarness, { items: [] });
    cleanup.push(m.destroy);
    expect(m.container.querySelectorAll('[data-component="atlas-legend"] > span')).toHaveLength(0);
  });
});
