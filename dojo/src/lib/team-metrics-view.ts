// The lead's team view (#307 R8, D38) as the screen shows it. Pure: the wire from
// GET …/team-metrics in, the rows out. Each cell reads the aggregate that fits its
// metric (TEAM_METRICS' `read`); an absent metric is a dash, never a zero — the
// database returned nothing the reader may see.
import { TEAM_METRICS, type TeamMetricDef, type TeamMetricsWire } from './team-metrics';
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

export interface TeamMetricsView {
	window: string;
	columns: readonly TeamMetricDef[];
	teams: Array<{ id: string; name: string; members: TeamMemberRow[] }>;
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

export function toTeamMetricsView(wire: TeamMetricsWire): TeamMetricsView {
	return {
		window: `last ${wire.days} days, since ${wire.since}`,
		columns: TEAM_METRICS,
		teams: wire.teams.map((t) => ({
			id: t.id,
			name: t.name,
			members: t.members.map((m) => {
				const days = Object.values(m.metrics).map((a) => a.last_on);
				return {
					principalId: m.principal_id,
					label: personLabel({ principalId: m.principal_id, name: m.display_name, email: m.email }),
					you: m.you,
					cells: TEAM_METRICS.map((def) => {
						const agg = m.metrics[def.key];
						return { key: def.key, text: agg ? formatMetric(agg[def.read], def.as) : DASH };
					}),
					last: days.length ? days.sort().at(-1)! : null
				};
			})
		}))
	};
}
