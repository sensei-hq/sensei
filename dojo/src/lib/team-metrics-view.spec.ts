// The lead's team view (#307 R8, D38) as the screen shows it: one row per member,
// one cell per shown metric — the aggregate that fits it (a count totals, a rate
// averages), formatted; an absent metric is a dash, never a zero.
import { describe, it, expect } from 'vitest';
import { toTeamMetricsView, formatMetric } from './team-metrics-view';
import { TEAM_METRICS, type TeamMetricsWire } from './team-metrics';

const agg = (total: number, mean: number, last_on = '2026-10-21') => ({ total, mean, days: 3, repositories: 2, last_on });

const wire: TeamMetricsWire = {
	since: '2026-10-16',
	days: 7,
	teams: [
		{
			id: 'tmA',
			name: 'Payments',
			members: [
				{
					principal_id: 'lena-0000-uuid',
					display_name: 'Lena',
					email: null,
					you: true,
					metrics: { throughput: agg(12, 4), ftr: agg(2.4, 0.8), tokens_per_day: agg(1_250_000, 400_000, '2026-10-20') }
				},
				{ principal_id: 'mo-00000-uuid', display_name: null, email: 'mo@x.co', you: false, metrics: {} }
			]
		}
	]
};

describe('formatMetric', () => {
	it('prints a count compactly and a rate as a whole percent', () => {
		expect(formatMetric(12, 'count')).toBe('12');
		expect(formatMetric(1_250_000, 'count')).toBe('1.3M');
		expect(formatMetric(4_200, 'count')).toBe('4.2k');
		expect(formatMetric(0.8, 'percent')).toBe('80%');
		expect(formatMetric(0.123, 'percent')).toBe('12%');
	});
});

describe('toTeamMetricsView', () => {
	const view = toTeamMetricsView(wire);

	it('heads the columns with the shown metrics, in order', () => {
		expect(view.columns.map((c) => c.label)).toEqual(TEAM_METRICS.map((m) => m.label));
	});

	it("reads each metric's own aggregate: sessions total, first-try averages", () => {
		const lena = view.teams[0].members[0];
		expect(lena.cells.map((c) => c.text)).toEqual(['12', '80%', '—', '1.3M']);
		expect(lena.you).toBe(true);
		expect(lena.last).toBe('2026-10-21');
	});

	it('a member with nothing readable shows dashes and no last day — never zeros', () => {
		const mo = view.teams[0].members[1];
		expect(mo.label).toBe('mo@x.co');
		expect(mo.cells.every((c) => c.text === '—')).toBe(true);
		expect(mo.last).toBeNull();
	});

	it('names the window', () => {
		expect(view.window).toBe('last 7 days, since 2026-10-16');
	});
});
