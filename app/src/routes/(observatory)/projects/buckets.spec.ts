import { describe, it, expect } from 'vitest';
import { bucketProjects, isWarning, type EnrichedProject } from './buckets.js';

const p = (over: Partial<EnrichedProject> = {}): EnrichedProject => ({
  id: over.id ?? 'p',
  name: over.name ?? 'unknown',
  maturity: 'learning',
  stack: {},
  ftr14d: 0,
  sessions7d: 0,
  warn: false,
  ...over,
} as EnrichedProject);

describe('bucketProjects', () => {
  it('active bucket carries anything with sessions in the last 7 days', () => {
    const b = bucketProjects([
      p({ id: '1', name: 'sensei', sessions7d: 5 }),
      p({ id: '2', name: 'rokkit', sessions7d: 0 }),
    ]);
    expect(b.active.map(x => x.id)).toEqual(['1']);
    expect(b.recent.map(x => x.id)).toEqual(['2']);
  });

  it('active is sorted by sessions7d desc, ties by name asc', () => {
    const b = bucketProjects([
      p({ id: '1', name: 'zulu',  sessions7d: 3 }),
      p({ id: '2', name: 'alpha', sessions7d: 3 }),
      p({ id: '3', name: 'mike',  sessions7d: 8 }),
    ]);
    expect(b.active.map(x => x.name)).toEqual(['mike', 'alpha', 'zulu']);
  });

  it('recent is sorted alphabetically', () => {
    const b = bucketProjects([
      p({ id: '1', name: 'zulu' }),
      p({ id: '2', name: 'alpha' }),
      p({ id: '3', name: 'mike' }),
    ]);
    expect(b.recent.map(x => x.name)).toEqual(['alpha', 'mike', 'zulu']);
  });

  it('archived flag routes projects into the archived bucket', () => {
    const b = bucketProjects([
      { ...p({ id: '1', name: 'old', sessions7d: 2 }), archived: true } as EnrichedProject,
      p({ id: '2', name: 'now', sessions7d: 4 }),
    ]);
    expect(b.archived.map(x => x.id)).toEqual(['1']);
    expect(b.active  .map(x => x.id)).toEqual(['2']);
  });

  it('archived bucket is empty when no projects carry the flag (today)', () => {
    const b = bucketProjects([p({ id: '1', name: 'x', sessions7d: 1 })]);
    expect(b.archived).toEqual([]);
  });
});

describe('isWarning', () => {
  it('flags any open drift', () => {
    expect(isWarning({ open_drift_count: 1, ftr_7d: 1 })).toBe(true);
  });

  it('flags a 7-day FTR below 60%', () => {
    expect(isWarning({ open_drift_count: 0, ftr_7d: 0.59 })).toBe(true);
  });

  it('stays quiet when both signals are healthy', () => {
    expect(isWarning({ open_drift_count: 0, ftr_7d: 0.6 })).toBe(false);
    expect(isWarning({ open_drift_count: 0, ftr_7d: 1   })).toBe(false);
  });
});
