// Rate-limited attempts (#311, generalised in #308 R5b): an action a person may
// take only so many times in a window — a failed invite accept, an Azure DevOps
// connect step. Taken atomically by dojo.take_attempt (count + record in one
// call, serialised per person and action — database/tests/dojo/request_attempts.sql);
// a failed call is an error, never a free pass.
import { describe, it, expect } from 'vitest';
import { fakeDojoDb } from './fake-dojo-db';
import { guardAttempt, RATE_LIMITS, releaseAttempt, takeAttempt } from './rate-limit';

function db(take: { rows: unknown[]; error?: { message: string } }) {
	return fakeDojoDb({
		'rpc:take_attempt': { rows: take.rows as never[], error: take.error },
		'rpc:release_attempt': { rows: [] }
	});
}

describe('rate limits', () => {
	it('names a limit and a window for each action', () => {
		expect(RATE_LIMITS.invite_accept).toEqual({ limit: 10, windowMs: 15 * 60 * 1000 });
		expect(RATE_LIMITS.azure_connect.limit).toBeGreaterThan(0);
	});

	it('takes an attempt in one call, with the action\'s limit and window', async () => {
		const d = db({ rows: ['attempt-1'] });
		expect(await takeAttempt(d as never, 'p1', 'azure_connect')).toBe('attempt-1');
		expect(d.rpcCalls).toEqual([
			{
				fn: 'take_attempt',
				args: { p_principal: 'p1', p_action: 'azure_connect', p_limit: RATE_LIMITS.azure_connect.limit, p_window: '900 seconds' }
			}
		]);
	});

	it('none left is null; guardAttempt says 429', async () => {
		const d = db({ rows: [null] });
		expect(await takeAttempt(d as never, 'p1', 'azure_connect')).toBeNull();
		await expect(guardAttempt(d as never, 'p1', 'azure_connect')).rejects.toMatchObject({ status: 429 });
	});

	it('releases an attempt', async () => {
		const d = db({ rows: ['attempt-1'] });
		await releaseAttempt(d as never, 'attempt-1');
		expect(d.rpcCalls).toEqual([{ fn: 'release_attempt', args: { p_id: 'attempt-1' } }]);
	});

	it('a failed take is an error, not a free pass', async () => {
		await expect(takeAttempt(db({ rows: [], error: { message: 'boom' } }) as never, 'p1', 'azure_connect')).rejects.toMatchObject({
			status: 500
		});
	});
});
