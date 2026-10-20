import { readOutcome, type ApiResult } from '$lib/api.js';
import type { SessionData } from '$lib/types.js';

/** A session as the Replay picker lists it. */
export type ReplaySession = { id: string; task: string; startedAt: string; ftr?: number | null };

/** The Replay picker's sessions, or why they could not be read (#288) — not
 *  the empty list that read as "No sessions recorded yet". */
export async function loadReplaySessions(api: {
  tryGetSessions: () => Promise<ApiResult<Pick<SessionData, 'sessions'> & Partial<SessionData>>>;
}): Promise<{ sessions: ReplaySession[]; error: string | null }> {
  const { data, error } = readOutcome(await api.tryGetSessions());
  const sessions = (data?.sessions ?? []).map((s) => ({ id: s.id, task: s.task, startedAt: s.startedAt, ftr: s.ftr }));
  return { sessions, error };
}
