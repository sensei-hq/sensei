import type { SessionData } from '$lib/types.js';

/** The display contract RecentSessions.svelte consumes. */
export interface RecentSession {
  id: string;
  project: string;
  task: string;
  startedAt: string;
  completedAt?: string;
  ftr?: number | null;
}

type WireSession = SessionData['sessions'][number];

/**
 * Human label for a session row — never blank. Hook-derived sessions (#31)
 * carry no task, so fall back to the summary, then the outcome, then a generic
 * word, so the row never renders an empty middle column.
 */
export function sessionLabel(s: Pick<WireSession, 'task' | 'summary' | 'outcome'>): string {
  return s.task?.trim() || s.summary?.trim() || s.outcome?.trim() || 'session';
}

/**
 * Shape raw `/api/sessions` rows into the RecentSessions display contract:
 * drop contentless ghost rows (no project AND no label — which is what made the
 * observatory render blank rows, #61), give every surviving row a non-blank
 * label, normalize `ftr` to 0/1, and cap to `limit`. The daemon returns
 * newest-first, so order is preserved.
 */
export function toRecentSessions(
  sessions: SessionData['sessions'],
  limit: number,
): RecentSession[] {
  return sessions
    .filter((s) => (s.project ?? '').trim() || (s.task ?? '').trim() || (s.summary ?? '').trim())
    .slice(0, limit)
    .map((s) => ({
      id: s.id,
      project: (s.project ?? '').trim(),
      task: sessionLabel(s),
      startedAt: s.startedAt,
      completedAt: s.completedAt,
      ftr: s.ftr == null ? null : (s.ftr ? 1 : 0),
    }));
}
