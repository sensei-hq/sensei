// The developer console's metric cards: one per Teams-view metric, in its order,
// each with the window's number and a day-by-day line. A count sums over the
// window; a rate pools by its denominators across days (Σ value × denominator /
// Σ denominator), as the lead's view pools it — never a mean of daily rates. A
// day that cannot be pooled is left out, not read as zero.
import { describe, it, expect } from 'vitest';
import { toMyMetricsView } from './my-metrics-view';
import { TEAM_METRICS } from './team-metrics';
import type { MyMetricsWire } from './my-metrics';

const wire = (series: MyMetricsWire['series']): MyMetricsWire => ({ since: '2026-09-23', days: 30, series });

describe('toMyMetricsView', () => {
	it('has one card per Teams-view metric, in its order', () => {
		const view = toMyMetricsView(wire({}));
		expect(view.cards.map((c) => [c.key, c.label])).toEqual(TEAM_METRICS.map((m) => [m.key, m.label]));
		expect(view.days).toBe(30);
	});

	it('sums a count over the window and draws it day by day', () => {
		const card = toMyMetricsView(
			wire({
				throughput: [
					{ day: '2026-10-20', total: 4, weighted: null, denominator: null },
					{ day: '2026-10-21', total: 2, weighted: null, denominator: null }
				]
			})
		).cards.find((c) => c.key === 'throughput');
		expect(card).toMatchObject({ value: '6', points: [4, 2], empty: false });
	});

	it('pools a rate by its denominators across days, not a mean of the daily rates', async () => {
		// 0.5 over 4 and 1.0 over 2: pooled 4/6 = 67%; the mean of the rates would say 75%.
		const card = toMyMetricsView(
			wire({
				ftr: [
					{ day: '2026-10-20', total: 0.5, weighted: 0.5, denominator: 4 },
					{ day: '2026-10-21', total: 1, weighted: 1, denominator: 2 }
				]
			})
		).cards.find((c) => c.key === 'ftr');
		expect(card).toMatchObject({ value: '67%', points: [0.5, 1], empty: false });
	});

	it('leaves a rate day without a denominator out of the number and the line', () => {
		const card = toMyMetricsView(
			wire({
				ftr: [
					{ day: '2026-10-20', total: 3, weighted: null, denominator: null },
					{ day: '2026-10-21', total: 0.8, weighted: 0.8, denominator: 5 }
				]
			})
		).cards.find((c) => c.key === 'ftr');
		expect(card).toMatchObject({ value: '80%', points: [0.8] });
	});

	it('says a rate that cannot be pooled on any day is unknown, not zero', () => {
		const card = toMyMetricsView(
			wire({ ftr: [{ day: '2026-10-21', total: 3, weighted: null, denominator: null }] })
		).cards.find((c) => c.key === 'ftr');
		expect(card).toMatchObject({ value: '—', points: [], empty: false });
	});

	it('marks a metric nothing was sent for as empty', () => {
		const card = toMyMetricsView(wire({})).cards.find((c) => c.key === 'tokens_per_day');
		expect(card).toMatchObject({ value: '—', points: [], empty: true });
	});
});
