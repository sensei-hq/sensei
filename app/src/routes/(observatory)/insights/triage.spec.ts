import { describe, it, expect } from 'vitest';
import { triageBuckets, type TriageRec } from './triage.js';

const rec = (over: Partial<TriageRec> = {}): TriageRec => ({
  id: 'r',
  projectId: 'p',
  projectName: 'sensei',
  title: 'X',
  why: 'because',
  urgency: 'low',
  status: 'pending',
  actedAt: null,
  ...over,
});

describe('triageBuckets', () => {
  it('routes pending high → Now, pending medium → Soon', () => {
    const b = triageBuckets([
      rec({ id: '1', urgency: 'high',   status: 'pending' }),
      rec({ id: '2', urgency: 'medium', status: 'pending' }),
    ]);
    expect(b.now .map(r => r.id)).toEqual(['1']);
    expect(b.soon.map(r => r.id)).toEqual(['2']);
  });

  it('routes accepted + dismissed → Settled, sorted by actedAt desc', () => {
    const b = triageBuckets([
      rec({ id: '1', status: 'accepted',  actedAt: '2026-06-30T10:00:00Z' }),
      rec({ id: '2', status: 'dismissed', actedAt: '2026-07-01T10:00:00Z' }),
      rec({ id: '3', status: 'accepted',  actedAt: '2026-06-29T10:00:00Z' }),
    ]);
    expect(b.settled.map(r => r.id)).toEqual(['2', '1', '3']);
  });

  it('drops low-urgency pending and superseded from every bucket', () => {
    const b = triageBuckets([
      rec({ id: '1', urgency: 'low',    status: 'pending'    }),
      rec({ id: '2', urgency: 'medium', status: 'superseded' }),
    ]);
    expect(b.now).toEqual([]);
    expect(b.soon).toEqual([]);
    expect(b.settled).toEqual([]);
  });

  it('caps Settled at 20 rows so the column stays readable', () => {
    const many = Array.from({ length: 30 }, (_, i) =>
      rec({ id: String(i), status: 'accepted', actedAt: `2026-06-${(i + 1).toString().padStart(2, '0')}T10:00:00Z` }),
    );
    const b = triageBuckets(many);
    expect(b.settled.length).toBe(20);
  });

  it('sorts Now and Soon by title asc (deterministic)', () => {
    const b = triageBuckets([
      rec({ id: 'a', title: 'zebra',  urgency: 'high', status: 'pending' }),
      rec({ id: 'b', title: 'apple',  urgency: 'high', status: 'pending' }),
    ]);
    expect(b.now.map(r => r.title)).toEqual(['apple', 'zebra']);
  });
});
