// A developer's own metrics, day by day. dojo.my_metric_series answers for the
// caller only (bound to current_principal_id, SECURITY INVOKER); this module asks
// for the window and shapes the answer into one series per metric the Teams view
// shows, so a person's card and their lead's column agree. A failed read is an
// error, never an empty chart.
import { describe, it, expect } from 'vitest';
import { fakeDojoDb, type FakeTable } from './fake-dojo-db';
import { readMyMetrics } from './my-metrics-data';
import { AdminError, type DojoClient } from './admin-data';

const TODAY = new Date('2026-10-22T09:00:00Z');

function db(series: FakeTable) {
	const f = fakeDojoDb({ 'rpc:my_metric_series': series });
	return f as unknown as DojoClient & typeof f;
}

describe('readMyMetrics', () => {
	it('shapes the rows into one series per metric, oldest day first', async () => {
		const d = db({
			rows: [
				{ metric_key: 'ftr', computed_on: '2026-10-21', total: '1.5', weighted: '0.6667', denominator: '6', repositories: 2 },
				{ metric_key: 'throughput', computed_on: '2026-10-21', total: 4, weighted: null, denominator: null, repositories: 1 },
				{ metric_key: 'ftr', computed_on: '2026-10-20', total: 3, weighted: null, denominator: null, repositories: 1 },
				{ metric_key: 'cyclomatic_density', computed_on: '2026-10-21', total: 9, weighted: null, denominator: null, repositories: 1 }
			]
		});
		const view = await readMyMetrics(d, { days: 30, today: TODAY });

		expect(d.rpcCalls).toEqual([{ fn: 'my_metric_series', args: { p_since: '2026-09-23' } }]);
		expect(view).toEqual({
			since: '2026-09-23',
			days: 30,
			series: {
				ftr: [
					{ day: '2026-10-20', total: 3, weighted: null, denominator: null },
					{ day: '2026-10-21', total: 1.5, weighted: 0.6667, denominator: 6 }
				],
				throughput: [{ day: '2026-10-21', total: 4, weighted: null, denominator: null }]
			}
		});
	});

	it('answers an empty series when nothing was sent', async () => {
		const view = await readMyMetrics(db({ rows: [] }), { days: 7, today: TODAY });
		expect(view.series).toEqual({});
	});

	it('a failed read is an error, never an empty chart', async () => {
		const d = db({ rows: [], error: { message: 'permission denied for function my_metric_series' } });
		await expect(readMyMetrics(d, { days: 7, today: TODAY })).rejects.toBeInstanceOf(AdminError);
	});

	it('a value that is not a number is a fault, not a zero', async () => {
		const d = db({
			rows: [{ metric_key: 'throughput', computed_on: '2026-10-21', total: 'NaN', weighted: null, denominator: null, repositories: 1 }]
		});
		await expect(readMyMetrics(d, { days: 7, today: TODAY })).rejects.toBeInstanceOf(AdminError);
	});
});
