// The ELECTION write path — the half of `sync_enabled` that had no writer.
//
// Who may elect, and which authority applies, are the database's:
// dojo.set_repository_election decides from the caller's own verdict row of
// dojo.all_my_repositories, writes the election and re-reads the verdict —
// proved against a real Postgres in database/tests/dojo/election_access.sql
// (#236). This side passes the request through and turns the database's
// refusal into the right status, never a softer one.
import { describe, it, expect } from 'vitest';
import { setElection } from './elections';
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

describe('setElection', () => {
	it('asks the database to elect THIS repository, and nothing about authority', async () => {
		// No authority parameter: if the caller could name it, any member of an
		// organization could write the ORGANIZATION's election.
		const verdict = {
			repo_key: 'github.com/alice/api',
			elected: true,
			authority: 'user',
			sync_enabled: true,
			reason_code: null
		};
		const { db, calls } = rpcDb({ data: verdict, error: null });
		const out = await setElection(db, 'github.com/alice/api', true);
		expect(calls).toEqual([
			{ fn: 'set_repository_election', args: { p_repo_key: 'github.com/alice/api', p_elected: true } }
		]);
		// The verdict re-read by the database, not an echo of the input.
		expect(out).toEqual(verdict);
	});

	for (const [code, status, why] of [
		['P0002', 404, 'not in the caller view — scoping IS the authorization'],
		['55000', 409, 'no authority yet: the forge visibility is not captured'],
		['42501', 403, 'a member who may not configure it']
	] as const) {
		it(`maps the database's ${code} to ${status} (${why}), keeping its message`, async () => {
			const { db } = rpcDb({ data: null, error: { code, message: `refused: ${why}` } });
			const err = await setElection(db, 'github.com/alice/api', true).catch((e) => e);
			expect(err).toBeInstanceOf(AdminError);
			expect((err as AdminError).status).toBe(status);
			expect((err as AdminError).message).toBe(`refused: ${why}`);
		});
	}

	it('fails closed (500) on any other database error — never a 404 "not yours"', async () => {
		// A 404 tells the caller the repository is not theirs, which is a different
		// and load-bearing claim.
		const { db } = rpcDb({ data: null, error: { code: '08006', message: 'connection reset' } });
		const err = await setElection(db, 'github.com/alice/api', true).catch((e) => e);
		expect((err as AdminError).status).toBe(500);
	});
});
