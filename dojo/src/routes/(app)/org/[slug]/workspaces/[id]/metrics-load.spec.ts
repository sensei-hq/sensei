// The org workspace drill-in's metrics (WP5): per-repository charts from the WP4
// read; a failed read is an error state, never empty charts (F1, #109).
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocks = vi.hoisted(() => ({
	getWorkspaceMetrics: vi.fn(),
	listWorkspaces: vi.fn(),
	loadWorkspaceConstitution: vi.fn()
}));
vi.mock('$lib/client-data', () => ({ listWorkspaces: mocks.listWorkspaces }));
vi.mock('$lib/workspace-constitution', () => ({
	loadWorkspaceConstitution: mocks.loadWorkspaceConstitution
}));
vi.mock('$lib/workspace-metrics-data', () => ({
	getWorkspaceMetrics: mocks.getWorkspaceMetrics
}));

const { load } = await import('./+page');

const ORG = {
	id: 'acme',
	kanji: '件',
	name: 'Acme',
	kind: 'Organization',
	url: 'organization/acme',
	role: 'admin',
	from: 'test'
};
const WS = {
	id: 'w1',
	tenant_id: 't-acme',
	slug: 'api',
	name: 'API',
	classification: 'internal',
	phase: 'build',
	last_run_at: null,
	runs_week: 0
};

const run = () =>
	load({
		params: { slug: 'acme', id: 'w1' },
		parent: async () => ({ memberships: [ORG], accessToken: 'jwt-A' }),
		fetch
	} as never) as Promise<Record<string, unknown>>;

beforeEach(() => {
	vi.clearAllMocks();
	mocks.listWorkspaces.mockResolvedValue([WS]);
	mocks.loadWorkspaceConstitution.mockResolvedValue({ ladder: [], conflicts: [], error: null });
});

describe('the workspace metrics loader', () => {
	it('reads the workspace series and shapes its per-repository view', async () => {
		mocks.getWorkspaceMetrics.mockResolvedValue({
			since: '2026-10-05',
			days: 30,
			rows: [
				{
					repository_id: 'r1',
					repo_key: 'github.com/acme/api',
					metric_key: 'throughput',
					scope: 'repo',
					principal_id: null,
					day: '2026-10-20',
					value: 4,
					props: {}
				}
			]
		});
		const data = await run();
		expect(mocks.getWorkspaceMetrics.mock.calls[0][0]).toBe('organization/acme');
		expect(mocks.getWorkspaceMetrics.mock.calls[0][1]).toBe('w1');
		expect(mocks.getWorkspaceMetrics.mock.calls[0][2]).toMatchObject({ accessToken: 'jwt-A' });
		expect(data.metricsError).toBeNull();
		const view = data.metricsView as {
			repos: { name: string; cards: { key: string; value: string }[] }[];
		};
		expect(view.repos.map((r) => r.name)).toEqual(['acme/api']);
		expect(view.repos[0].cards.find((c) => c.key === 'throughput')?.value).toBe('4');
	});

	it('a failed read is an error state, never empty charts', async () => {
		mocks.getWorkspaceMetrics.mockRejectedValue(new Error('connection reset'));
		const data = await run();
		expect(data.metricsView).toBeNull();
		expect(data.metricsError).toBe('connection reset');
	});
});
