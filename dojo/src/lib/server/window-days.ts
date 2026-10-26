// The window in days a metrics read takes from `?days=`: a whole number from 1 to
// the Teams view's ceiling, the fallback when absent, and a 400 otherwise — never
// a silently clamped window.
import { AdminError } from './admin-data';
import { TEAM_METRICS_MAX_DAYS } from '$lib/team-metrics';

export function windowDays(url: URL, fallback: number): number {
	const raw = url.searchParams.get('days');
	if (raw === null) return fallback;
	const days = /^\d+$/.test(raw) ? Number(raw) : NaN;
	if (!(days >= 1 && days <= TEAM_METRICS_MAX_DAYS)) {
		throw new AdminError(400, `days must be a whole number from 1 to ${TEAM_METRICS_MAX_DAYS}`);
	}
	return days;
}
