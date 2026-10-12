// callerDb reads AS the caller; tenantCallerDb is the daemon's tenant calls during
// the one-release window of #236 step 4 — the person's session when they present
// one, the service role only for the legacy device token (removed in 4e).
import { describe, it, expect, vi } from 'vitest';

const sessionClient = { as: 'session' };
const systemClient = { as: 'system' };
const userDb = vi.fn(() => sessionClient);
vi.mock('./dojo-supabase', async (importOriginal) => {
	const actual = await (importOriginal as () => Promise<Record<string, unknown>>)();
	return { ...actual, userDb, dojoDb: () => systemClient };
});

const { callerDb, tenantCallerDb } = await import('./caller-db');

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

describe('tenantCallerDb', () => {
	it('a session caller acts as themselves', () => {
		expect(tenantCallerDb({ accessToken: 'jwt' })).toBe(sessionClient);
		expect(userDb).toHaveBeenLastCalledWith('jwt');
	});

	it('a device-token caller (no session) is the system, for the window only', () => {
		expect(tenantCallerDb({})).toBe(systemClient);
	});
});
