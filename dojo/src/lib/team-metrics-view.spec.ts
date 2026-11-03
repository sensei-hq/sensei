// The Teams view (#307 R8, D38) as the screen shows it: per team, a totals row
// (every member) and — for a lead or admin — one row per member. Each cell reads
// the aggregate that fits its metric (a count totals, a rate pools by its
// denominator); an absent or unpoolable value is a dash, never a zero.
import { describe, it, expect } from 'vitest';
import { toTeamMetricsView, formatMetric } from './team-metrics-view';
import { TEAM_METRICS, type TeamMetricsWire } from './team-metrics';

const agg = (total: number, weighted: number | null, last_on = '2026-10-21') => ({
	total,
	weighted,
	days: 3,
	repositories: 2,
	last_on
});

const wire: TeamMetricsWire = {
	since: '2026-10-16',
	days: 7,
	per_person: true,
	k_min: 3,
	teams: [
		{
			id: 'tmA',
			name: 'Payments',
			member_count: 4,
			totals: {
				throughput: { total: 30, weighted: null, contributors: 3 },
				ftr: { total: 2.1, weighted: 0.7, contributors: 3 },
				rework_ratio: { total: 0.6, weighted: null, contributors: 3 }
			},
			members: [
				{
					principal_id: 'lena-0000-uuid',
					display_name: 'Lena',
					email: null,
					you: true,
					metrics: {
						throughput: agg(12, null),
						ftr: agg(2.4, 0.8),
						tokens_per_day: agg(1_250_000, null, '2026-10-20')
					}
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
		// A month of tokens runs to billions; "17738.8M" is a number nobody reads.
		expect(formatMetric(17_738_800_000, 'count')).toBe('17.7B');
		expect(formatMetric(0.8, 'percent')).toBe('80%');
		expect(formatMetric(0.123, 'percent')).toBe('12%');
	});
});

describe('toTeamMetricsView', () => {
	const view = toTeamMetricsView(wire);

	it('heads the columns with the shown metrics, in order', () => {
		expect(view.columns.map((c) => c.label)).toEqual(TEAM_METRICS.map((m) => m.label));
	});

	it("reads each metric's own aggregate: sessions total, first-try pooled", () => {
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

	it('the team row: totals and pooled rates; an unpoolable rate or a missing total is a dash', () => {
		expect(view.teams[0].totals.map((c) => c.text)).toEqual(['30', '70%', '—', '—']);
		expect(view.teams[0].memberCount).toBe(4);
	});

	it('says why a total may be missing — the k-anonymity floor, counted without the reader below lead', () => {
		expect(view.totalsNote).toBe('A team number shows once at least 3 people have sent it.');
		expect(toTeamMetricsView({ ...wire, per_person: false }).totalsNote).toBe(
			'A team number shows once at least 3 other people have sent it.'
		);
	});

	it('names the window and whether people are shown', () => {
		expect(view.window).toBe('last 7 days, since 2026-10-16');
		expect(view.perPerson).toBe(true);
		expect(toTeamMetricsView({ ...wire, per_person: false }).perPerson).toBe(false);
	});
});
