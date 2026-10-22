// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Harness from './HealthRadar.harness.svelte';
import type { HealthComponent } from '$lib/metrics/health-radar.js';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

const components: Record<string, HealthComponent> = {
  ftr: { name: 'First-turn resolution', rating: 5, weight: 3 },
  coverage: { name: 'Coverage', rating: 2, weight: 3 },
  spec_depth: { name: 'Spec depth', rating: 4, weight: 2 },
  churn_rate: { name: 'Churn rate', rating: 1, weight: 1 },
};

const mount = (props: Record<string, unknown>) => {
  const r = mountComponent(Harness as never, props as never);
  cleanup.push(r.destroy);
  return r;
};

describe('HealthRadar', () => {
  it('draws a plot and shows the composite score with its rated count', () => {
    const { container } = mount({ components, score: 65, ratedMetrics: 4 });

    expect(container.querySelector('[data-testid="health-radar-plot"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="health-score"]')?.textContent?.trim()).toBe('65');
    expect(container.textContent).toContain('4 rated');
    // The polar geom actually rendered, rather than the card sitting empty.
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('renders the quiet state instead of a collapsed radar when nothing is rated', () => {
    // A radar with no spokes would draw as a dot at the centre, which reads as
    // "every signal failing" when it means "not measured yet" (spec I1/I4).
    const { container } = mount({ components: {}, score: null, ratedMetrics: 0 });

    expect(container.querySelector('[data-testid="health-radar-empty"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="health-radar-plot"]')).toBeNull();
    expect(container.querySelector('[data-testid="health-score"]')).toBeNull();
  });

  it('omits the score readout entirely when the daemon reported none', () => {
    // Honest-empty: no score must never render as 0, which would read as a real
    // failing measurement.
    const { container } = mount({ components, score: null, ratedMetrics: 4 });

    expect(container.querySelector('[data-testid="health-score"]')).toBeNull();
    expect(container.textContent).not.toContain('0 rated');
    expect(container.querySelector('[data-testid="health-radar-plot"]')).not.toBeNull();
  });

  it('tones the score by band, not by a hardcoded colour', () => {
    const good = mount({ components, score: 90, ratedMetrics: 4 });
    expect(good.container.querySelector('[data-testid="health-score"]')?.className)
      .toContain('text-success');

    const bad = mount({ components, score: 20, ratedMetrics: 4 });
    expect(bad.container.querySelector('[data-testid="health-score"]')?.className)
      .toContain('text-danger');
  });
});
