import { describe, it, expect } from 'vitest';
import { toRecentSessions, sessionLabel } from './recent-sessions.js';
import type { SessionData } from '$lib/types.js';

type Wire = SessionData['sessions'][number];

const wire = (over: Partial<Wire> = {}): Wire => ({
  id: 's1',
  project: 'lumen',
  task: 'Fix auth',
  summary: '',
  outcome: 'completed',
  ftr: 1,
  startedAt: '2026-06-20T10:00:00Z',
  completedAt: '2026-06-20T10:30:00Z',
  ...over,
});

describe('sessionLabel', () => {
  it('prefers task, then summary, then outcome, then a generic word', () => {
    expect(sessionLabel({ task: 'do x', summary: 's', outcome: 'completed' })).toBe('do x');
    expect(sessionLabel({ task: '', summary: 'reset flow', outcome: 'completed' })).toBe('reset flow');
    expect(sessionLabel({ task: '  ', summary: '', outcome: 'corrected' })).toBe('corrected');
    expect(sessionLabel({ task: '', summary: '', outcome: '' })).toBe('session');
  });
});

describe('toRecentSessions', () => {
  it('passes a well-formed session through with its real fields', () => {
    const [s] = toRecentSessions([wire()], 4);
    expect(s).toMatchObject({
      id: 's1', project: 'lumen', task: 'Fix auth',
      startedAt: '2026-06-20T10:00:00Z', completedAt: '2026-06-20T10:30:00Z', ftr: 1,
    });
  });

  it('labels a task-less session from its summary so the row is never blank', () => {
    const [s] = toRecentSessions([wire({ task: '', summary: 'rotate tokens' })], 4);
    expect(s.task).toBe('rotate tokens');
  });

  it('drops contentless ghost rows (no project, no task, no summary)', () => {
    const rows = toRecentSessions(
      [wire({ id: 'ghost', project: null, task: '', summary: '', outcome: '' }), wire({ id: 'real' })],
      4,
    );
    expect(rows.map((r) => r.id)).toEqual(['real']);
  });

  it('keeps a session that has a project even when it has no task label of its own', () => {
    const [s] = toRecentSessions([wire({ task: '', summary: '', outcome: '', project: 'canvas' })], 4);
    expect(s.project).toBe('canvas');
    expect(s.task).toBe('session');
  });

  it('normalizes a boolean-ish ftr to 0/1 and preserves null', () => {
    // The wire sends ftr as a boolean; the UI compares with >= 1 / === 1.
    expect(toRecentSessions([wire({ ftr: true as unknown as number })], 4)[0].ftr).toBe(1);
    expect(toRecentSessions([wire({ ftr: false as unknown as number })], 4)[0].ftr).toBe(0);
    expect(toRecentSessions([wire({ ftr: null })], 4)[0].ftr).toBeNull();
  });

  it('caps to the limit, preserving daemon (newest-first) order', () => {
    const many = Array.from({ length: 10 }, (_, i) => wire({ id: `s${i}` }));
    const rows = toRecentSessions(many, 4);
    expect(rows).toHaveLength(4);
    expect(rows.map((r) => r.id)).toEqual(['s0', 's1', 's2', 's3']);
  });
});
