// Rate-limited attempts (#311, generalised in #308 R5b): an action a person may
// take only so many times in a window — a failed invite accept, an Azure DevOps
// connect. Counted per person and action in dojo.request_attempts; a failed
// count is an error, never a free pass.
import { describe, it, expect } from 'vitest';
import { fakeDojoDb } from './fake-dojo-db';
import { attemptsLeft, recordAttempt, RATE_LIMITS, guardAttempt } from './rate-limit';

const NOW = 1_800_000_000_000;

function db(rows: Record<string, unknown>[] = [], error?: { message: string }) {
	return fakeDojoDb({ request_attempts: { rows, error } });
}

describe('rate limits', () => {
	it('names a limit and a window for each action', () => {
		expect(RATE_LIMITS.invite_accept).toEqual({ limit: 10, windowMs: 15 * 60 * 1000 });
		expect(RATE_LIMITS.azure_connect.limit).toBeGreaterThan(0);
	});

	it("counts only this person's attempts at this action in the window", async () => {
		const recent = new Date(NOW - 60_000).toISOString();
		const old = new Date(NOW - 60 * 60 * 1000).toISOString();
		const d = db([
			{ principal_id: 'p1', action: 'azure_connect', at: recent },
			{ principal_id: 'p1', action: 'azure_connect', at: recent },
			{ principal_id: 'p1', action: 'azure_connect', at: old },
			{ principal_id: 'p1', action: 'invite_accept', at: recent },
			{ principal_id: 'p2', action: 'azure_connect', at: recent }
		]);
		expect(await attemptsLeft(d as never, 'p1', 'azure_connect', NOW)).toBe(RATE_LIMITS.azure_connect.limit - 2);
	});

	it('records an attempt', async () => {
		const d = db();
		await recordAttempt(d as never, 'p1', 'azure_connect');
		expect(d.tables.request_attempts.rows).toEqual([expect.objectContaining({ principal_id: 'p1', action: 'azure_connect' })]);
	});

	it('guardAttempt records one and refuses with 429 once none are left', async () => {
		const recent = new Date(NOW - 1000).toISOString();
		const full = db(
			Array.from({ length: RATE_LIMITS.azure_connect.limit }, () => ({ principal_id: 'p1', action: 'azure_connect', at: recent }))
		);
		await expect(guardAttempt(full as never, 'p1', 'azure_connect', NOW)).rejects.toMatchObject({ status: 429 });
		expect(full.tables.request_attempts.rows).toHaveLength(RATE_LIMITS.azure_connect.limit);
		const empty = db();
		await guardAttempt(empty as never, 'p1', 'azure_connect', NOW);
		expect(empty.tables.request_attempts.rows).toHaveLength(1);
	});

	it('a failed count is an error, not a free pass', async () => {
		await expect(attemptsLeft(db([], { message: 'boom' }) as never, 'p1', 'azure_connect', NOW)).rejects.toMatchObject({
			status: 500
		});
	});
});
