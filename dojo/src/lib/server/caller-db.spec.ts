// callerDb reads AS the caller, and refuses a caller with no session (#236).
import { describe, it, expect, vi } from 'vitest';

const sessionClient = { as: 'session' };
const systemClient = { as: 'system' };
const userDb = vi.fn(() => sessionClient);
vi.mock('./dojo-supabase', async (importOriginal) => {
	const actual = await (importOriginal as () => Promise<Record<string, unknown>>)();
	return { ...actual, userDb, dojoDb: () => systemClient };
});

const { callerDb } = await import('./caller-db');

describe('callerDb', () => {
	it('reads with the caller’s JWT', () => {
		expect(callerDb({ accessToken: 'jwt' })).toBe(sessionClient);
		expect(userDb).toHaveBeenLastCalledWith('jwt');
	});

	it('refuses a caller with no session (401), never the service role', () => {
		const err = (() => {
			try {
				callerDb({});
			} catch (e) {
				return e;
			}
		})();
		expect(err).toBeInstanceOf(Response);
		expect((err as Response).status).toBe(401);
	});
});
