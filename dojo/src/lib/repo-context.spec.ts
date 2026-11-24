// Unit tests for the shared repository drill-in context (`loadRepoContext`,
// #532): the org from the route against real memberships, the repository by
// id from the caller's repos and belonging to this tenant, the branch from
// ?branch= when tracked else the tracked default. Unknown org/repo redirects
// out; the diagrams and repo pages (and later the workspace screen) share it.
import { describe, it, expect, vi } from 'vitest';
import { loadRepoContext } from './repo-context';

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

const ctx = (over: Record<string, unknown> = {}) => ({
	slug: 'acme',
	id: 'r1',
	branch: null as string | null,
	memberships: [ORG],
	accessToken: 'jwt-A',
	fetch: fetch as typeof fetch,
	listRepos: vi.fn(async () => [REPO]),
	...over
});

describe('loadRepoContext', () => {
	it('resolves org, repo and the default branch', async () => {
		const out = await loadRepoContext(ctx());
		expect(out).toEqual({
			slug: 'acme',
			orgName: 'Acme',
			tenantKey: 'organization/acme',
			repo: {
				id: 'r1',
				key: 'github.com/acme/api',
				name: 'api',
				branches: REPO.graph_branches,
				configurable: true,
				last_activity_at: null
			},
			branch: 'main'
		});
	});

	it('a ?branch= naming a tracked branch wins; anything else falls back to the default', async () => {
		expect((await loadRepoContext(ctx({ branch: 'dev' }))).branch).toBe('dev');
		expect((await loadRepoContext(ctx({ branch: 'nope' }))).branch).toBe('main');
	});

	it('an unknown repo id, another tenant’s repo, or a failed read redirects to the org page', async () => {
		await expect(loadRepoContext(ctx({ id: 'nope' }))).rejects.toMatchObject({ status: 307, location: '/org/acme' });
		await expect(
			loadRepoContext(ctx({ listRepos: async () => [{ ...REPO, tenant: 'organization/other' }] }))
		).rejects.toMatchObject({ status: 307 });
		await expect(
			loadRepoContext(ctx({ listRepos: async () => { throw new Error('down'); } }))
		).rejects.toMatchObject({ status: 307 });
	});

	it('a non-member org redirects to /you', async () => {
		await expect(loadRepoContext(ctx({ slug: 'nope' }))).rejects.toMatchObject({ status: 307, location: '/you' });
	});
});
