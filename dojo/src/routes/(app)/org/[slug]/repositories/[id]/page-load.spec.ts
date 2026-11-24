// The org repository loader (#532): the shared repo context plus the repo's
// metric series over the window. A failed metrics read is carried as an
// error, never as empty charts; an empty window reads as empty.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocks = vi.hoisted(() => ({ listMyRepos: vi.fn(), getRepoMetrics: vi.fn() }));
vi.mock('$lib/client-data', () => ({
	listMyRepos: mocks.listMyRepos,
	getRepoMetrics: mocks.getRepoMetrics,
	ClientApiError: class extends Error {}
}));

const { load } = await import('./+page');

const ORG = { id: 'acme', kanji: '監', name: 'Acme', kind: 'Organization', url: 'organization/acme', role: 'admin', from: 'github' };
const REPO = {
	repository_id: 'r1',
	repo_key: 'github.com/acme/api',
	name: 'api',
	tenant: 'organization/acme',
	graph_branches: [{ branch: 'main', is_default: true }],
	configurable_by_me: true
};

function run(query = '') {
	return load({
		params: { slug: 'acme', id: 'r1' },
		url: new URL(`http://x/?${query}`),
		parent: async () => ({ memberships: [ORG], accessToken: 'jwt-A' }),
		fetch
	} as never) as Promise<Record<string, unknown>>;
}

beforeEach(() => {
	vi.clearAllMocks();
	mocks.listMyRepos.mockResolvedValue([REPO]);
	mocks.getRepoMetrics.mockResolvedValue({ since: '2026-10-05', days: 30, rows: [] });
});

describe('the repository loader', () => {
	it('resolves the repo and reads its metrics over the default window', async () => {
		const data = await run();
		expect(data.repo).toMatchObject({ id: 'r1', key: 'github.com/acme/api' });
		expect(mocks.getRepoMetrics).toHaveBeenCalledWith(
			'organization/acme',
			'r1',
			expect.objectContaining({ accessToken: 'jwt-A' })
		);
		expect(data.metricsError).toBeNull();
	});

	it('?days= overrides the window', async () => {
		await run('days=7');
		expect(mocks.getRepoMetrics.mock.calls[0][2]).toMatchObject({ days: 7 });
	});

	it('a failed metrics read is an error, never empty charts', async () => {
		mocks.getRepoMetrics.mockRejectedValue(new Error('the request failed'));
		const data = await run();
		expect(data.metricsView).toBeNull();
		expect(data.metricsError).toBe('the request failed');
	});
});
