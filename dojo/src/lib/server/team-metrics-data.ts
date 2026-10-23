// The Teams view (#307 R8, D38): every member reads their teams' totals; a lead
// also reads the people on the teams they are on, an admin every team and its
// people.
//
// Who sees whose numbers is the DATABASE's decision, not this module's:
//   * dojo.team_member_metrics runs as the caller (SECURITY INVOKER), so
//     repository_metrics' policy decides which per-person rows count;
//   * dojo.team_metric_totals answers team totals over the caller's teams (every
//     team for an admin), on repositories the caller can read, and only where at
//     least k_anonymity_min() people sent the metric.
// This module picks which teams to list and whether to ask per person (lead and
// admin), and shapes the answer. Fails closed: a failed read is an error, never an
// empty view.
import { AdminError, type DojoClient } from './admin-data';
import { K_ANONYMITY_MIN_CONTRIBUTORS } from './artifacts-data';
import { listTeams } from './teams-data';
import { callFn } from './tenant-rows';
import { TEAM_METRIC_KEYS, type TeamMetricAgg, type TeamMetricsWire, type TeamTotal } from '$lib/team-metrics';

/** The first day of a `days`-long window ending `today` (UTC), as `YYYY-MM-DD`. */
export function windowSince(today: Date, days: number): string {
	const start = new Date(Date.UTC(today.getUTCFullYear(), today.getUTCMonth(), today.getUTCDate() - (days - 1)));
	return start.toISOString().slice(0, 10);
}

type Num = number | string;

interface PersonRow {
	principal_id: string;
	metric_key: string;
	total: Num;
	weighted: Num | null;
	days: number;
	repositories: number;
	last_on: string;
}

interface TotalRow {
	team_id: string;
	metric_key: string;
	total: Num;
	weighted: Num | null;
	contributors: number;
}

/** A numeric the database sent as text (Postgres `numeric` over PostgREST) is a
 *  number here; anything that is not one is a fault, not a zero. */
function num(v: Num): number {
	const n = typeof v === 'number' ? v : Number(v);
	if (!Number.isFinite(n)) throw new AdminError(500, `team metrics: not a number: ${String(v)}`);
	return n;
}

const numOrNull = (v: Num | null): number | null => (v === null ? null : num(v));

/** Group rows into `Map<owner, Record<metric, value>>`. */
function byKey<R extends { metric_key: string }, V>(rows: R[], owner: (r: R) => string, value: (r: R) => V) {
	const out = new Map<string, Record<string, V>>();
	for (const r of rows) {
		const mine = out.get(owner(r)) ?? {};
		mine[r.metric_key] = value(r);
		out.set(owner(r), mine);
	}
	return out;
}

/** Lead and admin read per person; below that, totals only. */
const PER_PERSON_ROLES = new Set(['lead', 'admin']);

/** `db` is the caller's client — every read that decides access runs as them.
 *  `nameDb` names the teammates the caller already read (see listTeams). */
export async function readTeamMetrics(
	db: DojoClient,
	nameDb: DojoClient,
	caller: { tenantId: string; userId: string; role: string },
	opts: { days: number; today: Date }
): Promise<TeamMetricsWire> {
	const since = windowSince(opts.today, opts.days);
	const perPerson = PER_PERSON_ROLES.has(caller.role);
	const base = { since, days: opts.days, per_person: perPerson, k_min: K_ANONYMITY_MIN_CONTRIBUTORS };

	const all = await listTeams(db, caller.tenantId, nameDb);
	const teams =
		caller.role === 'admin' ? all : all.filter((t) => t.members.some((m) => m.principal_id === caller.userId));
	if (teams.length === 0) return { ...base, teams: [] };

	const keys = [...TEAM_METRIC_KEYS];
	const totalRows =
		(await callFn<TotalRow[] | null>(db, 'team_metric_totals', {
			p_tenant: caller.tenantId,
			p_keys: keys,
			p_since: since
		})) ?? [];
	const totals = byKey(
		totalRows,
		(r) => r.team_id,
		(r): TeamTotal => ({ total: num(r.total), weighted: numOrNull(r.weighted), contributors: r.contributors })
	);

	let people = new Map<string, Record<string, TeamMetricAgg>>();
	const ids = [...new Set(teams.flatMap((t) => t.members.map((m) => m.principal_id)))];
	if (perPerson && ids.length > 0) {
		const rows =
			(await callFn<PersonRow[] | null>(db, 'team_member_metrics', {
				p_tenant: caller.tenantId,
				p_principals: ids,
				p_keys: keys,
				p_since: since
			})) ?? [];
		people = byKey(
			rows,
			(r) => r.principal_id,
			(r): TeamMetricAgg => ({
				total: num(r.total),
				weighted: numOrNull(r.weighted),
				days: r.days,
				repositories: r.repositories,
				last_on: r.last_on
			})
		);
	}

	return {
		...base,
		teams: teams.map((t) => ({
			id: t.id,
			name: t.name,
			member_count: t.members.length,
			totals: totals.get(t.id) ?? {},
			members: perPerson
				? t.members.map((m) => ({
						principal_id: m.principal_id,
						display_name: m.display_name,
						email: m.email,
						you: m.principal_id === caller.userId,
						metrics: people.get(m.principal_id) ?? {}
					}))
				: []
		}))
	};
}
