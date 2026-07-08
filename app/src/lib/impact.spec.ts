import { describe, it, expect } from 'vitest';
import {
  bucketImpact,
  formatDeltaPct,
  pct,
  verdictMeta,
  verdictToneClass,
  VERDICT_META,
  type ImpactRow,
} from './impact.js';

const row = (over: Partial<ImpactRow> = {}): ImpactRow => ({
  id: 'r',
  projectId: 'p',
  projectName: 'sensei',
  title: 'x',
  status: 'accepted',
  actionType: 'create_agent',
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

  it('buckets a real all-pending project (the sensei case) into pending only', () => {
    const b = bucketImpact([
      row({ id: '1', verdict: 'pending', baselineFtr: null, currentFtr: null, ftrDelta: null }),
      row({ id: '2', verdict: 'pending', baselineFtr: null, currentFtr: null, ftrDelta: null }),
    ]);
    expect(b.pending.map(r => r.id)).toEqual(['1', '2']);
    expect(b.positive).toHaveLength(0);
    expect(b.negative).toHaveLength(0);
    expect(b.neutral).toHaveLength(0);
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

describe('pct', () => {
  it('rounds a fractional rate to a whole percent', () => {
    expect(pct(0.666)).toBe('67%');
    expect(pct(1)).toBe('100%');
    expect(pct(0)).toBe('0%');
  });
  it('renders "—" when the value is unknown', () => {
    expect(pct(null)).toBe('—');
  });
});

describe('verdictMeta', () => {
  it('maps each known verdict to its glyph + tone + label', () => {
    expect(verdictMeta('positive')).toEqual(VERDICT_META.positive);
    expect(verdictMeta('neutral')).toEqual(VERDICT_META.neutral);
    expect(verdictMeta('negative')).toEqual(VERDICT_META.negative);
    expect(verdictMeta('pending')).toEqual(VERDICT_META.pending);
  });
  it('tones: positive→success, negative→warning, neutral/pending→ink', () => {
    expect(verdictMeta('positive').tone).toBe('success');
    expect(verdictMeta('negative').tone).toBe('warning');
    expect(verdictMeta('neutral').tone).toBe('ink');
    expect(verdictMeta('pending').tone).toBe('ink');
  });
  it('defaults an unknown or absent verdict to pending', () => {
    expect(verdictMeta('bogus')).toEqual(VERDICT_META.pending);
    expect(verdictMeta(null)).toEqual(VERDICT_META.pending);
    expect(verdictMeta(undefined)).toEqual(VERDICT_META.pending);
  });
});

describe('verdictToneClass', () => {
  it('maps tone + kind to a named-token utility', () => {
    expect(verdictToneClass('success')).toBe('text-success');
    expect(verdictToneClass('warning', 'bg')).toBe('bg-warning');
    expect(verdictToneClass('ink', 'border')).toBe('border-ink-mute');
  });
  it('defaults kind to text', () => {
    expect(verdictToneClass('warning')).toBe('text-warning');
  });
});
