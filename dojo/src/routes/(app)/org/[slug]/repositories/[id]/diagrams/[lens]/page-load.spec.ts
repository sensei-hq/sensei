// The org repository-diagrams loader (#268): the org from the route, the
// repository by id from the caller's repos (tenant-scoped by the slug),
// the lens validated, the branch defaulted to the tracked default. Unknown
// org/repo redirects out; an unknown lens is a 404, never a default lens.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocks = vi.hoisted(() => ({ listMyRepos: vi.fn() }));
vi.mock('$lib/client-data', () => ({
	listMyRepos: mocks.listMyRepos,
	ClientApiError: class extends Error {}
}));

const { load } = await import('./+page');

const ORG = { id: 'acme', kanji: '監', name: 'Acme', kind: 'Organization', url: 'organization/acme', role: 'admin', from: 'github' };
const REPO = {
	repository_id: 'r1',
	repo_key: 'github.com/acme/api',
	name: 'api',
	tenant: 'organization/acme',
	graph_branches: [
		{ branch: 'main', is_default: true },
		{ branch: 'dev', is_default: false }
	],
	configurable_by_me: true
};

function run(params: Record<string, string>, query = '') {
	return load({
		params,
		url: new URL(`http://x/?${query}`),
		parent: async () => ({ memberships: [{ ...ORG, role: 'admin' }], accessToken: 'jwt-A' }),
		fetch
	} as never) as Promise<Record<string, unknown>>;
}

const params = { slug: 'acme', id: 'r1', lens: 'structure' };

beforeEach(() => {
	vi.clearAllMocks();
	mocks.listMyRepos.mockResolvedValue([REPO]);
});

describe('the repository-diagrams loader', () => {
	it('resolves org, repo, lens and the default branch', async () => {
		const data = await run(params);
		expect(data.orgName).toBe('Acme');
		expect(data.lens).toBe('structure');
		expect(data.repo).toMatchObject({ id: 'r1', key: 'github.com/acme/api', name: 'api', configurable: true });
		expect(data.branch).toBe('main');
		expect((data.repo as { branches: unknown }).branches).toEqual([
			{ branch: 'main', is_default: true },
			{ branch: 'dev', is_default: false }
		]);
	});

	it('a ?branch= naming a tracked branch wins over the default', async () => {
		expect((await run(params, 'branch=dev')).branch).toBe('dev');
	});

	it('a ?branch= naming nothing tracked falls back to the default', async () => {
		expect((await run(params, 'branch=nope')).branch).toBe('main');
	});

	it('an unknown lens is a 404', async () => {
		await expect(run({ ...params, lens: 'nope' })).rejects.toMatchObject({ status: 404 });
	});

	it('an unknown repo id redirects to the org page', async () => {
		await expect(run({ ...params, id: 'nope' })).rejects.toMatchObject({ status: 307 });
	});

	it('a repo of another tenant redirects to the org page', async () => {
		mocks.listMyRepos.mockResolvedValue([{ ...REPO, tenant: 'organization/other' }]);
		await expect(run(params)).rejects.toMatchObject({ status: 307 });
	});

	it('a non-member org redirects to /you', async () => {
		await expect(run({ ...params, slug: 'nope' })).rejects.toMatchObject({ status: 307 });
	});
});
