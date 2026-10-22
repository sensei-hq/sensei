// The lead's team view (#307 R8, D38): the teams the caller is ON, each member
// with their per-person metrics over a window.
//
// Who sees whose numbers is the DATABASE's decision, not this module's:
// dojo.team_member_metrics runs as the caller (SECURITY INVOKER), so
// repository_metrics' policy (can_read_repository_metric) decides which rows
// count — a lead their teammates' rows on repositories the lead can read, anyone
// else only their own. This module picks whom to ask about (the caller's
// teammates — a lead on no team asks about no one) and shapes the answer.
// Fails closed: a failed read is an error, never an empty view.
import { AdminError, type DojoClient } from './admin-data';
import { listTeams } from './teams-data';
import { callFn } from './tenant-rows';
import { TEAM_METRIC_KEYS, type TeamMetricAgg, type TeamMetricsWire } from '$lib/team-metrics';

/** The first day of a `days`-long window ending `today` (UTC), as `YYYY-MM-DD`. */
export function windowSince(today: Date, days: number): string {
	const start = new Date(Date.UTC(today.getUTCFullYear(), today.getUTCMonth(), today.getUTCDate() - (days - 1)));
	return start.toISOString().slice(0, 10);
}

interface MetricRow {
	principal_id: string;
	metric_key: string;
	total: number | string;
	mean: number | string;
	days: number;
	repositories: number;
	last_on: string;
}

/** A numeric the database sent as text (Postgres `numeric` over PostgREST) is a
 *  number here; anything that is not one is a fault, not a zero. */
function num(v: number | string): number {
	const n = typeof v === 'number' ? v : Number(v);
	if (!Number.isFinite(n)) throw new AdminError(500, `team metrics: not a number: ${String(v)}`);
	return n;
}

/** `db` is the caller's client — every read that decides access runs as them.
 *  `nameDb` names the teammates the caller already read (see listTeams). */
export async function readTeamMetrics(
	db: DojoClient,
	nameDb: DojoClient,
	caller: { tenantId: string; userId: string },
	opts: { days: number; today: Date }
): Promise<TeamMetricsWire> {
	const since = windowSince(opts.today, opts.days);
	const teams = (await listTeams(db, caller.tenantId, nameDb)).filter((t) =>
		t.members.some((m) => m.principal_id === caller.userId)
	);
	const people = [...new Set(teams.flatMap((t) => t.members.map((m) => m.principal_id)))];
	if (people.length === 0) return { since, days: opts.days, teams: [] };

	const rows =
		(await callFn<MetricRow[] | null>(db, 'team_member_metrics', {
			p_tenant: caller.tenantId,
			p_principals: people,
			p_keys: [...TEAM_METRIC_KEYS],
			p_since: since
		})) ?? [];

	const byPerson = new Map<string, Record<string, TeamMetricAgg>>();
	for (const r of rows) {
		const mine = byPerson.get(r.principal_id) ?? {};
		mine[r.metric_key] = {
			total: num(r.total),
			mean: num(r.mean),
			days: r.days,
			repositories: r.repositories,
			last_on: r.last_on
		};
		byPerson.set(r.principal_id, mine);
	}

	return {
		since,
		days: opts.days,
		teams: teams.map((t) => ({
			id: t.id,
			name: t.name,
			members: t.members.map((m) => ({
				principal_id: m.principal_id,
				display_name: m.display_name,
				email: m.email,
				you: m.principal_id === caller.userId,
				metrics: byPerson.get(m.principal_id) ?? {}
			}))
		}))
	};
}
