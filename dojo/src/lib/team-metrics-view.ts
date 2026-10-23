// The Teams view (#307 R8, D38) as the screen shows it. Pure: the wire from
// GET …/team-metrics in, the rows out. Each cell reads the aggregate that fits its
// metric (TEAM_METRICS' `read`: a count totals, a rate pools by its denominator);
// an absent or unpoolable value is a dash, never a zero.
import {
	TEAM_METRICS,
	type TeamMetricDef,
	type TeamMetricValue,
	type TeamMetricsWire
} from './team-metrics';
import { personLabel } from './viewers-view';

export interface TeamMetricCell {
	key: string;
	text: string;
}

export interface TeamMemberRow {
	principalId: string;
	label: string;
	you: boolean;
	cells: TeamMetricCell[];
	/** The latest day any shown metric arrived for; null when none did. */
	last: string | null;
}

export interface TeamRow {
	id: string;
	name: string;
	memberCount: number;
	/** The team's totals, one cell per column. */
	totals: TeamMetricCell[];
	/** Per person — empty below lead. */
	members: TeamMemberRow[];
}

export interface TeamMetricsView {
	window: string;
	perPerson: boolean;
	/** Why a team number may be missing: the k-anonymity floor. */
	totalsNote: string;
	columns: readonly TeamMetricDef[];
	teams: TeamRow[];
}

const DASH = '—';

/** A count compactly (12 · 4.2k · 1.3M); a rate (0..1) as a whole percent. */
export function formatMetric(value: number, as: TeamMetricDef['as']): string {
	if (as === 'percent') return `${Math.round(value * 100)}%`;
	const abs = Math.abs(value);
	if (abs >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
	if (abs >= 1_000) return `${(value / 1_000).toFixed(1)}k`;
	return String(Math.round(value));
}

/** One cell per shown metric, reading the aggregate that fits it. */
function cellsFor(values: Record<string, TeamMetricValue>): TeamMetricCell[] {
	return TEAM_METRICS.map((def) => {
		const v = values[def.key]?.[def.read];
		return { key: def.key, text: v == null ? DASH : formatMetric(v, def.as) };
	});
}

export function toTeamMetricsView(wire: TeamMetricsWire): TeamMetricsView {
	return {
		window: `last ${wire.days} days, since ${wire.since}`,
		perPerson: wire.per_person,
		totalsNote: `A team number shows once at least ${wire.k_min} people have sent it.`,
		columns: TEAM_METRICS,
		teams: wire.teams.map((t) => ({
			id: t.id,
			name: t.name,
			memberCount: t.member_count,
			totals: cellsFor(t.totals),
			members: t.members.map((m) => {
				const days = Object.values(m.metrics).map((a) => a.last_on);
				return {
					principalId: m.principal_id,
					label: personLabel({ principalId: m.principal_id, name: m.display_name, email: m.email }),
					you: m.you,
					cells: cellsFor(m.metrics),
					last: days.length ? days.sort().at(-1)! : null
				};
			})
		}))
	};
}
