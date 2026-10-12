// The USER-SESSION client (#236 step 3): reads that RLS decides, as the
// signed-in person. It is built from the PUBLIC anon key plus the caller's own
// JWT — never the service key — so a policy, not route code, is what limits the
// rows. A missing token is an error, never a silent fall back to the service
// client (which would quietly bypass every policy).
import { describe, it, expect, vi, beforeEach } from 'vitest';

const created: Array<{ url: string; key: string; opts: Record<string, unknown> }> = [];
vi.mock('@supabase/supabase-js', () => ({
	createClient: (url: string, key: string, opts: Record<string, unknown>) => {
		created.push({ url, key, opts });
		return { tag: 'user-client' };
	}
}));
vi.mock('$env/dynamic/public', () => ({
	env: { PUBLIC_SUPABASE_URL: 'http://sb.test', PUBLIC_SUPABASE_ANON_KEY: 'anon-key' }
}));
vi.mock('$env/dynamic/private', () => ({ env: { SUPABASE_SERVICE_ROLE_KEY: 'service-key' } }));

const { userDb } = await import('./dojo-supabase');

beforeEach(() => {
	created.length = 0;
});

describe('userDb — the signed-in person, under RLS', () => {
	it('is the anon key plus the caller\'s JWT, on the dojo schema', async () => {
		userDb('user-jwt');
		expect(created).toHaveLength(1);
		const { url, key, opts } = created[0];
		expect(url).toBe('http://sb.test');
		expect(key).toBe('anon-key');
		expect(key).not.toBe('service-key');
		expect((opts.db as { schema: string }).schema).toBe('dojo');
		const token = await (opts.accessToken as () => Promise<string>)();
		expect(token).toBe('user-jwt');
	});

	it('refuses to build without a token rather than fall back to the service client', () => {
		expect(() => userDb('')).toThrow(/token/i);
		expect(created).toHaveLength(0);
	});
});
