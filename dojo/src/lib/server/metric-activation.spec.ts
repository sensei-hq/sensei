// The ACTIVATION write path: a tenant switching a catalogue metric off for one
// repository.
//
// Who may decide, and the off-is-a-row / on-is-absence invariant, are the
// database's: dojo.set_metric_activation decides from the caller's own verdict
// row and a personal-or-admin hold on the tenant's cost decision — proved
// against a real Postgres in database/tests/dojo/metrics_write_access.sql
// (#236). This side passes the request through and keeps the refusal's status.
import { describe, it, expect } from 'vitest';
import { setMetricActivation } from './metric-activation';
import { AdminError } from './admin-data';

function rpcDb(result: { data: unknown; error: unknown }) {
	const calls: { fn: string; args: Record<string, unknown> }[] = [];
	const db = {
		rpc: (fn: string, args: Record<string, unknown>) => {
			calls.push({ fn, args });
			return { single: async () => result };
		}
	};
	return { db: db as never, calls };
}

describe('setMetricActivation', () => {
	it('asks the database to switch THIS metric for THIS repository, and returns its ruling', async () => {
		const ruling = { repo_key: 'github.com/acme/api', metric: 'churn', enabled: false, tenant: 'organization/acme' };
		const { db, calls } = rpcDb({ data: ruling, error: null });
		const out = await setMetricActivation(db, 'github.com/acme/api', 'churn', false);
		expect(calls).toEqual([
			{
				fn: 'set_metric_activation',
				args: { p_repo_key: 'github.com/acme/api', p_metric_key: 'churn', p_enabled: false }
			}
		]);
		expect(out).toEqual(ruling);
	});

	it('keeps a refusal as the status it means (403 not theirs to decide)', async () => {
		const { db } = rpcDb({ data: null, error: { code: '42501', message: 'you may not change metric activation' } });
		const err = await setMetricActivation(db, 'github.com/acme/api', 'churn', false).catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect((err as AdminError).status).toBe(403);
	});

	it('keeps an unknown repository or metric as 404', async () => {
		const { db } = rpcDb({ data: null, error: { code: 'P0002', message: 'unknown metric nope' } });
		const err = await setMetricActivation(db, 'github.com/acme/api', 'nope', false).catch((e) => e);
		expect((err as AdminError).status).toBe(404);
		expect((err as AdminError).message).toBe('unknown metric nope');
	});

	it('fails closed (500) on any other database error rather than reporting success', async () => {
		const { db } = rpcDb({ data: null, error: { code: '08006', message: 'connection reset' } });
		const err = await setMetricActivation(db, 'github.com/acme/api', 'churn', false).catch((e) => e);
		expect((err as AdminError).status).toBe(500);
	});
});
