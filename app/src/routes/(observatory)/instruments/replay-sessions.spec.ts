import { describe, it, expect } from 'vitest';
import { loadReplaySessions } from './replay-sessions';

// #288: the Replay picker's sessions, or why they could not be read — not the
// "No sessions recorded yet" an empty fallback list showed.
describe('loadReplaySessions', () => {
  it('keeps each session\'s id, task, start and FTR', async () => {
    const api = { tryGetSessions: async () => ({ ok: true as const, data: { stats: null, sessions: [
      { id: 's1', task: 'Fix auth', workspace: 'p', startedAt: '2026-10-20T08:00:00Z', ftr: 1, outcome: 'completed' },
    ] } }) };
    expect(await loadReplaySessions(api)).toEqual({
      sessions: [{ id: 's1', task: 'Fix auth', startedAt: '2026-10-20T08:00:00Z', ftr: 1 }],
      error: null,
    });
  });

  it('a failed read is an error, with no sessions', async () => {
    const api = { tryGetSessions: async () => ({ ok: false as const, error: { status: 503, message: 'Service Unavailable' } }) };
    expect(await loadReplaySessions(api)).toEqual({ sessions: [], error: 'Service Unavailable' });
  });
});
