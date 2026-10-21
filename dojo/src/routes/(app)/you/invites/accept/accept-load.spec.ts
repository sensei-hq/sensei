// The invite accept landing (F3b, #307 D8): a member lands in the dōjō they
// joined; a viewer is not a member — the org would bounce them — so they land on
// their projects, where the projects they are granted appear.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const acceptInvite = vi.fn();
vi.mock('$lib/client-data', () => ({
	acceptInvite,
	ClientApiError: class ClientApiError extends Error {}
}));
vi.mock('@sveltejs/kit', () => ({ redirect: (s: number, l: string) => { throw new Error(`${s} ${l}`); } }));

const { load } = await import('./+page.js');

const run = (token = 'tk') =>
	(load as unknown as (e: unknown) => Promise<unknown>)({
		url: new URL(`https://d/you/invites/accept?token=${token}`),
		fetch,
		parent: async () => ({ accessToken: 'jwt' })
	});

beforeEach(() => acceptInvite.mockReset());

describe('invite accept landing', () => {
	it('a member lands in the dōjō they joined', async () => {
		acceptInvite.mockResolvedValue({ tenant_id: 't1', role: 'contributor', access: 'member' });
		await expect(run()).rejects.toThrow('303 /org/t1');
	});

	it('a viewer lands on their projects, not in the dōjō', async () => {
		acceptInvite.mockResolvedValue({ tenant_id: 't1', role: null, access: 'viewer' });
		await expect(run()).rejects.toThrow('303 /you/projects');
	});
});
