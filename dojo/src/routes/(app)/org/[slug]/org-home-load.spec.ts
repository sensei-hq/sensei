// #241: the org home's "needs a maintainer" band comes from the tenant's open
// triage queue (maintainer floor), never from the kit fixtures.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const listTriage = vi.fn();
const listWorkspaces = vi.fn();
vi.mock('$lib/triage-data', () => ({ listTriage }));
vi.mock('$lib/client-data', () => ({
	listWorkspaces,
	DojoApiError: class DojoApiError extends Error {}
}));
vi.mock('@sveltejs/kit', () => ({ redirect: (s: number, l: string) => { throw new Error(`${s} ${l}`); } }));

const { load } = await import('./+page.js');

const ORG = { id: 'acme', name: 'Acme', url: 'organization/acme', role: 'maintainer', members: 4, pending: 2 };
const ROW = {
	signature: 's1', artifact_id: 'a1', kind: 'pattern', title: 'Idempotency', owner_scope: { label: 'Payments' },
	confidence: 0.8, contributor_count: 2, similarity: null, nearest_artifact_id: null, state: 'queued',
	created_at: new Date().toISOString()
};
const run = (role: string | null) =>
	(load as unknown as (e: unknown) => Promise<Record<string, unknown>>)({
		params: { slug: 'acme' },
		fetch,
		parent: async () => ({ memberships: [{ ...ORG, role }], accessToken: 'jwt' })
	});

beforeEach(() => {
	listWorkspaces.mockReset().mockResolvedValue([]);
	listTriage.mockReset().mockResolvedValue([ROW]);
});

describe('org home loader — the needs band', () => {
	it('a maintainer sees the open triage queue, not fixtures', async () => {
		const data = await run('maintainer');
		expect(listTriage).toHaveBeenCalledWith('organization/acme', expect.objectContaining({ accessToken: 'jwt' }));
		expect(data.needs).toEqual([expect.objectContaining({ id: 's1', title: 'Idempotency', dojo: 'Acme' })]);
		expect(data.needsError).toBeNull();
	});

	it('below maintainer the band is not theirs: nothing is asked, nothing shown', async () => {
		const data = await run('contributor');
		expect(listTriage).not.toHaveBeenCalled();
		expect(data.needs).toEqual([]);
	});

	it('a failed read is an error, not an empty or invented band', async () => {
		listTriage.mockRejectedValue(new Error('500'));
		const data = await run('maintainer');
		expect(data.needs).toEqual([]);
		expect(data.needsError).toBeTruthy();
	});

	// The membership's role is the server LABEL ('Maintainer', from ROLE_LABEL in
	// dojo-orgs.ts), not the rank key. The band must load for every label at or
	// above maintainer — the D69 "needs you" queue depends on it.
	it.each(['Maintainer', 'Lead', 'Admin'])('a real %s label loads the band', async (label) => {
		const data = await run(label);
		expect(listTriage).toHaveBeenCalledOnce();
		expect(data.needs).toEqual([expect.objectContaining({ id: 's1' })]);
	});

	it.each(['Contributor', 'Member'])('a real %s label asks nothing', async (label) => {
		await run(label);
		expect(listTriage).not.toHaveBeenCalled();
	});

	it('no role at all (a viewer, not a member) asks nothing', async () => {
		await run(null);
		expect(listTriage).not.toHaveBeenCalled();
	});
});
