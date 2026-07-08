// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import AdoptedCardHarness from './AdoptedCard.harness.svelte';
import type { ObservatoryAdopted } from '$lib/types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const item = (over: Partial<ObservatoryAdopted> = {}): ObservatoryAdopted => ({
  when: '2d ago',
  what: 'Canvas smoothing pattern → rule',
  scope: 'lumen-studio',
  source: 'from session s-2890',
  ...over,
});

describe('AdoptedCard', () => {
  it('renders the when, scope and what fields', () => {
    const m = mountComponent(AdoptedCardHarness, { item: item() });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('2d ago');
    expect(m.container.textContent).toContain('lumen-studio');
    expect(m.container.textContent).toContain('Canvas smoothing pattern → rule');
  });

  it('renders the source line when present', () => {
    const m = mountComponent(AdoptedCardHarness, { item: item() });
    cleanup.push(m.destroy);
    const src = m.container.querySelector('[data-adopted-source]');
    expect(src?.textContent).toContain('from session s-2890');
  });

  it('omits the source line when the source is empty', () => {
    const m = mountComponent(AdoptedCardHarness, { item: item({ source: '' }) });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-adopted-source]')).toBeNull();
  });
});
