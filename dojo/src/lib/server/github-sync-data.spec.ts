import { describe, it, expect, vi } from 'vitest';
import {
	fetchGithubOrgLogins,
	syncGithubMemberships,
	AdminError,
	type DojoClient
} from './github-sync-data';

function fakeFetch(status: number, body: unknown) {
	const fn = vi.fn(async () => ({ ok: status >= 200 && status < 300, status, json: async () => body }));
	return fn as unknown as typeof fetch;
}

describe('fetchGithubOrgLogins — reads the user orgs with their own token, fail-closed', () => {
	it('returns the org logins on 200', async () => {
		const logins = await fetchGithubOrgLogins('tok', fakeFetch(200, [{ login: 'acme' }, { login: 'globex' }]));
		expect(logins).toEqual(['acme', 'globex']);
	});
	it('drops non-string logins', async () => {
		expect(await fetchGithubOrgLogins('tok', fakeFetch(200, [{ login: 'acme' }, {}, { login: 42 }]))).toEqual(['acme']);
	});
	it('throws (never a fabricated list) on a non-2xx', async () => {
		await expect(fetchGithubOrgLogins('tok', fakeFetch(401, {}))).rejects.toBeInstanceOf(AdminError);
	});
	it('sends the bearer token', async () => {
		const fn = fakeFetch(200, []);
		await fetchGithubOrgLogins('tok-123', fn);
		const headers = (fn as unknown as ReturnType<typeof vi.fn>).mock.calls[0][1].headers;
		expect(headers.Authorization).toBe('Bearer tok-123');
	});
});

// A stub keyed by table: tenants read (.in→resolve), memberships read (.is→resolve),
// membership inserts (addMember: .single→resolve). Captures the joined inserts.
function makeDb(tenants: unknown[], existing: unknown[]) {
	const inserts: Record<string, unknown>[] = [];
	let table: string | undefined;
	const b: Record<string, unknown> = {};
	b.from = (t: string) => {
		table = t;
		return b;
	};
	b.select = () => b;
	b.insert = (p: Record<string, unknown>) => {
		inserts.push(p);
		return b;
	};
	b.in = () => Promise.resolve({ data: tenants, error: null });
	b.eq = () => b;
	b.is = () => Promise.resolve({ data: existing, error: null });
	b.single = () => Promise.resolve({ data: { id: 'm', role: 'contributor' }, error: null });
	return { db: b as unknown as DojoClient, inserts };
}

describe('syncGithubMemberships — provision only proven, not-already-joined orgs', () => {
	it('joins the matching github/{org} tenants the caller is not yet in', async () => {
		const { db, inserts } = makeDb(
			[{ id: 't-acme', key: 'github/acme' }, { id: 't-globex', key: 'github/globex' }],
			[{ tenant_id: 't-acme' }] // already in acme
		);
		const out = await syncGithubMemberships(db, 'u1', ['acme', 'globex']);
		expect(out.joined).toEqual(['github/globex']); // only the new one
		expect(inserts).toHaveLength(1);
		expect(inserts[0]).toMatchObject({
			tenant_id: 't-globex',
			user_id: 'u1',
			role: 'contributor',
			kind: 'employer',
			authenticated_via: 'github_oauth'
		});
	});
	it('provisions nothing when no org dōjō exists (never invents a tenant)', async () => {
		const { db, inserts } = makeDb([], []);
		expect((await syncGithubMemberships(db, 'u1', ['acme'])).joined).toEqual([]);
		expect(inserts).toHaveLength(0);
	});
	it('is a no-op for an empty org list (no queries)', async () => {
		const { db, inserts } = makeDb([{ id: 'x', key: 'github/x' }], []);
		expect((await syncGithubMemberships(db, 'u1', [])).joined).toEqual([]);
		expect(inserts).toHaveLength(0);
	});
});
