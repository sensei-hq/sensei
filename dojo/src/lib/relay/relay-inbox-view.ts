// Pure inbox view logic — rank + filter over RelaySession[]. No state, no DOM, no
// runes: the state slice composes these in `$derived`, and they're unit-tested
// directly. Semantics mirror the shipped relay-map (needs → attention → running →
// pending → done) so the sort order is unchanged by the rebuild.
import type { RelaySession, RelayFilter } from './types';

/** Sort key: what waits on you first, then stalled/blocked/failed, then running,
 *  then still-pending, then finished. Lower = higher in the list. */
export function rankSession(s: RelaySession): number {
	if (s.needs > 0) return 0;
	if (s.attention) return 1;
	if (s.status === 'running') return 2;
	if (s.status === 'done') return 4;
	return 3;
}

/** A stable-sorted copy by rank (Array.sort is stable in modern engines; ties keep
 *  input order = the load's own recency ordering). */
export function sortByRank(list: RelaySession[]): RelaySession[] {
	return [...list].sort((a, b) => rankSession(a) - rankSession(b));
}

export function matchesFilter(s: RelaySession, f: RelayFilter): boolean {
	switch (f) {
		case 'needs':
			return s.needs > 0 || s.attention !== null;
		case 'running':
			return s.status === 'running';
		case 'finished':
			return s.status === 'done';
		case 'all':
			return true;
	}
}

/** The shown list for a filter, ranked. */
export function filterSessions(list: RelaySession[], f: RelayFilter): RelaySession[] {
	return sortByRank(list.filter((s) => matchesFilter(s, f)));
}

/** Total open asks across all sessions — the "{n} need you" count. */
export function needsCount(list: RelaySession[]): number {
	return list.reduce((n, s) => n + s.needs, 0);
}
