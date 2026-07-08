import { describe, it, expect, vi } from 'vitest';
import type { SessionRow } from '$lib/types.js';
import { projectSessionsFetcher } from './project-sessions.js';

const wire = (o: Partial<SessionRow> = {}): SessionRow => ({
  id: 's1',
  project: 'sensei',
  task: 'Fix auth',
  summary: null,
  outcome: 'completed',
  ftr: true,
  turns: 10,
  corrections: 0,
  startedAt: new Date().toISOString(),
  completedAt: null,
  agent: 'claude',
  ...o,
});

describe('projectSessionsFetcher', () => {
  it('scopes /api/sessions to the project id and passes the range through', async () => {
    const rows = [wire({ id: 'a' }), wire({ id: 'b' })];
    const getSessionsDigest = vi.fn(async () => ({ sessions: rows }));
    const fetcher = projectSessionsFetcher({ getSessionsDigest }, 'proj-42');

    const out = await fetcher('30d');

    expect(getSessionsDigest).toHaveBeenCalledWith('30d', 'proj-42');
    expect(out.map((s) => s.id)).toEqual(['a', 'b']);
  });

  it('unwraps the digest envelope to the raw rows array', async () => {
    const getSessionsDigest = vi.fn(async () => ({ sessions: [] as SessionRow[] }));
    const fetcher = projectSessionsFetcher({ getSessionsDigest }, 'proj-1');
    await expect(fetcher('7d')).resolves.toEqual([]);
    expect(getSessionsDigest).toHaveBeenCalledWith('7d', 'proj-1');
  });
});
