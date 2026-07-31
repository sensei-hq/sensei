// The Health screen's contributions-vs-approvals weekly series — the bar chart
// that rendered empty (the health rollup carried no time series). Buckets
// `dojo.audit_events` over the last `weeks` 7-day windows: publish/distribute
// events are contributions, approve events are approvals. Kept separate from the
// 4-count `getHealth` rollup so neither read disturbs the other. Alerts + the
// leak-guard-blocks signal still wait on the containment seam (action
// 'contained'/'held', health.md Q2) — not built here.
import { AdminError, type DojoClient } from './admin-data';

export type { DojoClient };

/** One week's contributions/approvals pair (wire shape; the client maps it 1:1
 *  onto the presentational KitHealthWeek). */
export interface HealthWeek {
	wk: string;
	c: number;
	a: number;
}

const DAY_MS = 86_400_000;

/**
 * Bucket audit events into `weeks` 7-day windows ending at `now` (pure). Oldest
 * window is `W1`, most recent is `W{weeks}`. An `approve` action counts as an
 * approval; any other counted action (publish/distribute) is a contribution.
 * Events outside the window (or in the future) are ignored.
 */
export function bucketContribApprove(
	events: { ts: string; action: string }[],
	now: Date,
	weeks = 4
): HealthWeek[] {
	const buckets: HealthWeek[] = Array.from({ length: weeks }, (_, i) => ({ wk: `W${i + 1}`, c: 0, a: 0 }));
	for (const e of events) {
		const daysAgo = Math.floor((now.getTime() - new Date(e.ts).getTime()) / DAY_MS);
		if (daysAgo < 0) continue;
		const weeksAgo = Math.floor(daysAgo / 7);
		if (weeksAgo >= weeks) continue;
		const idx = weeks - 1 - weeksAgo; // most-recent → last bucket
		if (/approve/i.test(e.action)) buckets[idx].a++;
		else buckets[idx].c++;
	}
	return buckets;
}

/** The counted actions: contributions (publish/distribute) + approvals. */
const SERIES_ACTIONS = ['publish', 'distribute', 'approve'];

/**
 * Read + bucket the tenant's contributions-vs-approvals over the last `weeks`
 * weeks. One tenant-scoped audit-events read; fails closed (AdminError 500).
 */
export async function getContribVsApprove(
	db: DojoClient,
	tenantId: string,
	now: Date = new Date(),
	weeks = 4
): Promise<HealthWeek[]> {
	const windowStart = new Date(now.getTime() - weeks * 7 * DAY_MS).toISOString();
	const { data, error } = await db
		.from('audit_events')
		.select('ts, action')
		.eq('tenant_id', tenantId)
		.in('action', SERIES_ACTIONS)
		.gte('ts', windowStart);
	if (error) throw new AdminError(500, error.message);
	return bucketContribApprove((data ?? []) as { ts: string; action: string }[], now, weeks);
}
