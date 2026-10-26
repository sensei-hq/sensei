// The personal console's My metrics section: the loader reads the caller's own
// series and carries a failure SEPARATELY from an empty month (F1, #109).
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocks = vi.hoisted(() => ({ getMyMetrics: vi.fn() }));
// Every client call the loader imports; only getMyMetrics runs on this section.
vi.mock('$lib/client-data', () => ({
	getMyMetrics: mocks.getMyMetrics,
	listUserWorkspaces: vi.fn(),
	listContributions: vi.fn(),
	listLibraryPacks: vi.fn(),
	listMyRepos: vi.fn(),
	getMicrosoftLink: vi.fn(),
	ClientApiError: class extends Error {}
}));

const { load } = await import('./+page');

const run = () =>
	load({
		params: { section: 'metrics' },
		parent: async () => ({ memberships: [], accessToken: 'jwt-A' }),
		fetch
	} as never) as Promise<Record<string, unknown>>;

beforeEach(() => vi.clearAllMocks());

describe('the My metrics section loader', () => {
	it('reads the caller\'s series and shapes its cards', async () => {
		mocks.getMyMetrics.mockResolvedValue({
			since: '2026-09-23',
			days: 30,
			series: { throughput: [{ day: '2026-10-21', total: 4, weighted: null, denominator: null }] }
		});
		const data = await run();
		expect(mocks.getMyMetrics.mock.calls[0][0]).toMatchObject({ accessToken: 'jwt-A' });
		expect(data.metricsError).toBeNull();
		const view = data.metricsView as { cards: { key: string; value: string }[] };
		expect(view.cards.find((c) => c.key === 'throughput')?.value).toBe('4');
	});

	it('a failed read is an error, never an empty month', async () => {
		mocks.getMyMetrics.mockRejectedValue(new Error('the request failed'));
		const data = await run();
		expect(data.metricsView).toBeNull();
		expect(data.metricsError).toBe('the request failed');
	});
});
