import { describe, it, expect } from 'vitest';
import { bucketImpact, formatDeltaPct, type ImpactRow } from './aggregate.js';

const row = (over: Partial<ImpactRow> = {}): ImpactRow => ({
  id: 'r',
  projectId: 'p',
  projectName: 'sensei',
  title: 'x',
  status: 'accepted',
  verdict: 'positive',
  baselineFtr: 0.6,
  currentFtr: 0.7,
  ftrDelta: 0.1,
  reasoning: null,
  ...over,
});

describe('bucketImpact', () => {
  it('routes each row into its verdict bucket', () => {
    const b = bucketImpact([
      row({ id: '1', verdict: 'positive' }),
      row({ id: '2', verdict: 'negative' }),
      row({ id: '3', verdict: 'neutral'  }),
      row({ id: '4', verdict: 'pending'  }),
    ]);
    expect(b.positive.map(r => r.id)).toEqual(['1']);
    expect(b.negative.map(r => r.id)).toEqual(['2']);
    expect(b.neutral .map(r => r.id)).toEqual(['3']);
    expect(b.pending .map(r => r.id)).toEqual(['4']);
  });

  it('within a bucket, orders by |ftrDelta| desc so the loudest row leads', () => {
    const b = bucketImpact([
      row({ id: '1', ftrDelta: 0.02 }),
      row({ id: '2', ftrDelta: 0.20 }),
      row({ id: '3', ftrDelta: 0.10 }),
    ]);
    expect(b.positive.map(r => r.id)).toEqual(['2', '3', '1']);
  });

  it('null ftrDelta sinks to the bottom of the bucket', () => {
    const b = bucketImpact([
      row({ id: '1', ftrDelta: null }),
      row({ id: '2', ftrDelta: 0.05 }),
    ]);
    expect(b.positive.map(r => r.id)).toEqual(['2', '1']);
  });
});

describe('formatDeltaPct', () => {
  it('renders +N% for positive, -N% for negative', () => {
    expect(formatDeltaPct(0.14)).toBe('+14%');
    expect(formatDeltaPct(-0.03)).toBe('-3%');
  });
  it('renders ±0% when the delta rounds to zero (not misleading +0%)', () => {
    expect(formatDeltaPct(0)).toBe('±0%');
    expect(formatDeltaPct(0.004)).toBe('±0%');
  });
  it('renders "—" when the delta is unknown', () => {
    expect(formatDeltaPct(null)).toBe('—');
  });
});
