// Unit tests for the per-repository metrics read (`readRepoMetrics`, #532):
// one repository's daily series over the window, scoped to the addressed
// tenant through the caller's own `my_repositories` rows (principal pinned
// in the database). Outside the tenant is 404; a failed read is 500, never [].
import { describe, it, expect } from 'vitest';
import { readRepoMetrics } from './repo-metrics-data';
import { AdminError } from './admin-data';

function makeDb(opts: { mine: unknown[]; mineError?: { message: string }; series: unknown[] }) {
	const calls: string[] = [];
	const rpc = (fn: string, args: Record<string, unknown>) => {
		calls.push(fn);
		if (fn === 'my_repositories') {
			if (opts.mineError) throw new AdminError(500, opts.mineError.message);
			const chain: Record<string, unknown> = {};
			chain.select = () => chain;
			chain.eq = () => Promise.resolve({ data: opts.mine, error: null });
			return chain;
		}
		if (fn === 'repository_metric_series') {
			return {
				select: () => ({
					order: () => ({ order: () => ({ order: () => ({ range: () => ({ data: opts.series, error: null }) }) }) })
				})
			};
		}
		throw new Error(`unexpected rpc ${fn} ${JSON.stringify(args)}`);
	};
	return { db: { rpc } as never, calls };
}

const MINE = [{ repository_id: 'r1', tenant_id: 't-acme' }];
const SERIES = [
	{ repository_id: 'r1', metric_key: 'ftr', scope: 'repo', principal_id: null, computed_on: '2026-11-03', grain: 'daily', value: 7, props: {} }
];

describe('readRepoMetrics', () => {
	it('returns the shaped series for a repository of this tenant', async () => {
		const { db, calls } = makeDb({ mine: MINE, series: SERIES });
		const out = await readRepoMetrics(db, 't-acme', 'r1', { days: 30, today: new Date('2026-11-03T12:00:00Z') });
		expect(out.rows).toEqual([
			{ repository_id: 'r1', metric_key: 'ftr', scope: 'repo', principal_id: null, day: '2026-11-03', value: 7, props: {} }
		]);
		expect(calls).toEqual(['my_repositories', 'repository_metric_series']);
	});

	it('404s a repository of another tenant before touching its rows', async () => {
		const { db, calls } = makeDb({ mine: MINE, series: SERIES });
		const err = await readRepoMetrics(db, 't-other', 'r1', { days: 30, today: new Date() }).catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect(err.status).toBe(404);
		expect(calls).toEqual(['my_repositories']);
	});

	it('a failed scope read is 500, never 404', async () => {
		const { db } = makeDb({ mine: [], mineError: { message: 'boom' }, series: SERIES });
		const err = await readRepoMetrics(db, 't-acme', 'r1', { days: 30, today: new Date() }).catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect(err.status).toBe(500);
	});
});
