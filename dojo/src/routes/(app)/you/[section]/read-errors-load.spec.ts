// Rule packs and contributions: a failed read is carried as an error, never as an
// empty list — "you have none" and "we could not ask" are different answers
// (F1, #109). Both used to swallow the failure into [].
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocks = vi.hoisted(() => ({ listLibraryPacks: vi.fn(), listContributions: vi.fn() }));
vi.mock('$lib/client-data', () => ({
	listLibraryPacks: mocks.listLibraryPacks,
	listContributions: mocks.listContributions,
	getMyMetrics: vi.fn(),
	listUserWorkspaces: vi.fn(),
	listMyRepos: vi.fn(),
	getMicrosoftLink: vi.fn(),
	ClientApiError: class extends Error {}
}));

const { load } = await import('./+page');

const run = (section: string) =>
	load({
		params: { section },
		parent: async () => ({ memberships: [], accessToken: 'jwt-A' }),
		fetch
	} as never) as Promise<Record<string, unknown>>;

beforeEach(() => vi.clearAllMocks());

describe('a failed read on the personal sections', () => {
	it('rule packs: an error, not an empty library', async () => {
		mocks.listLibraryPacks.mockRejectedValue(new Error('the request failed'));
		const data = await run('packs');
		expect(data.packsError).toBe('the request failed');
		expect(data.rulePacks).toEqual([]);
	});

	it('rule packs: no error when the read lands', async () => {
		mocks.listLibraryPacks.mockResolvedValue({ packs: [], adopted: [] });
		expect((await run('packs')).packsError).toBeNull();
	});

	it('contributions: an error, not "nothing shared"', async () => {
		mocks.listContributions.mockRejectedValue(new Error('the request failed'));
		const data = await run('contributions');
		expect(data.contributionsError).toBe('the request failed');
	});

	it('contributions: no error when the read lands', async () => {
		mocks.listContributions.mockResolvedValue({ mine: [], downstream: [] });
		expect((await run('contributions')).contributionsError).toBeNull();
	});
});
