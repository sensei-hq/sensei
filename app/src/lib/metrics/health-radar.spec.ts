import { describe, it, expect } from 'vitest';
import {
  toSpokes,
  toAxes,
  scoreTone,
  weightCoverage,
  RATING_DOMAIN,
  type HealthComponent,
} from './health-radar.js';

const comps = (o: Record<string, Partial<HealthComponent>>): Record<string, HealthComponent> =>
  Object.fromEntries(
    Object.entries(o).map(([k, v]) => [k, { name: v.name ?? k, rating: v.rating ?? 0, weight: v.weight ?? 1 }]),
  );

describe('toSpokes', () => {
  it('orders heaviest first, then by label — a radar shape must be stable', () => {
    const spokes = toSpokes(
      comps({
        churn_rate: { name: 'Churn rate', rating: 1, weight: 1 },
        coverage: { name: 'Coverage', rating: 3, weight: 3 },
        spec_depth: { name: 'Spec depth', rating: 5, weight: 2 },
        ftr: { name: 'FTR', rating: 5, weight: 3 },
      }),
    );
    // weight 3 (Coverage, FTR — alphabetical within weight), then 2, then 1.
    expect(spokes.map((s) => s.label)).toEqual(['Coverage', 'FTR', 'Spec depth', 'Churn rate']);
  });

  it('is order-independent: the same components in any key order give the same shape', () => {
    const a = toSpokes(comps({ x: { rating: 1, weight: 2 }, y: { rating: 4, weight: 3 } }));
    const b = toSpokes(comps({ y: { rating: 4, weight: 3 }, x: { rating: 1, weight: 2 } }));
    expect(a).toEqual(b);
  });

  it('falls back to the metric key when the daemon sent no display name', () => {
    const [s] = toSpokes({ odd_metric: { name: '', rating: 2, weight: 1 } });
    expect(s.label).toBe('odd_metric');
  });

  it('drops entries with a non-numeric rating rather than plotting them as 0', () => {
    const spokes = toSpokes({
      good: { name: 'Good', rating: 3, weight: 1 },
      // A not-rated metric must never render as a spoke pinned at the centre —
      // that reads as "failing" when it means "not measured".
      bad: { name: 'Bad', rating: Number.NaN, weight: 1 },
    });
    expect(spokes.map((s) => s.metric)).toEqual(['good']);
  });

  it('tolerates an empty or missing components map', () => {
    expect(toSpokes({})).toEqual([]);
    expect(toSpokes(undefined as never)).toEqual([]);
  });
});

describe('toAxes', () => {
  it('mirrors the spoke order and pins every axis to the 0-5 rating domain', () => {
    const spokes = toSpokes(comps({ a: { rating: 1, weight: 3 }, b: { rating: 5, weight: 1 } }));
    const axes = toAxes(spokes);
    expect(axes.map((x) => x.key)).toEqual(spokes.map((s) => s.metric));
    // A shared, fixed domain is what makes spokes comparable; auto-scaling each
    // axis to its own data would make every metric look mid-range.
    expect(axes.every((x) => x.domain === RATING_DOMAIN)).toBe(true);
    expect(RATING_DOMAIN).toEqual([0, 5]);
  });
});

describe('scoreTone', () => {
  it('bands on the same 20-point steps the rating scale uses', () => {
    expect(scoreTone(100)).toBe('success');
    expect(scoreTone(80)).toBe('success');
    expect(scoreTone(79)).toBe('warning');
    expect(scoreTone(50)).toBe('warning');
    expect(scoreTone(49)).toBe('danger');
    expect(scoreTone(0)).toBe('danger');
  });
});

describe('weightCoverage', () => {
  it('reports the share of the model behind the score', () => {
    const spokes = toSpokes(comps({ a: { weight: 3 }, b: { weight: 2 } }));
    expect(weightCoverage(spokes, 29)).toBeCloseTo(5 / 29, 5);
  });

  it('never exceeds 1 and never divides by zero', () => {
    const spokes = toSpokes(comps({ a: { weight: 40 } }));
    expect(weightCoverage(spokes, 29)).toBe(1);
    expect(weightCoverage(spokes, 0)).toBe(0);
  });
});
