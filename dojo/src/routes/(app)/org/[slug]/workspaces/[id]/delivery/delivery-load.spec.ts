// The org workspace's Delivery page loader (tracker-delivery-metrics D5; #352): the
// org from real memberships, the workspace from the dōjō's real workspaces, and the
// delivery read — a failure carried as an error, never as "no boards".
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocks = vi.hoisted(() => ({ getWorkspaceDelivery: vi.fn(), listWorkspaces: vi.fn() }));
vi.mock('$lib/client-data', () => ({ listWorkspaces: mocks.listWorkspaces }));
vi.mock('$lib/workspace-delivery-data', () => ({ getWorkspaceDelivery: mocks.getWorkspaceDelivery }));

const { load } = await import('./+page');

const ORG = { id: 'acme', kanji: '件', name: 'Acme', kind: 'Organization', url: 'organization/acme', role: 'admin', from: 'test' };
const WS = { id: 'w1', tenant_id: 't-acme', slug: 'api', name: 'API', classification: 'internal', phase: 'build', last_run_at: null, runs_week: 0 };

const run = (memberships = [ORG]) =>
	load({
		params: { slug: 'acme', id: 'w1' },
		parent: async () => ({ memberships, accessToken: 'jwt-A' }),
		fetch
	} as never) as Promise<Record<string, unknown>>;

beforeEach(() => {
	vi.clearAllMocks();
	mocks.listWorkspaces.mockResolvedValue([WS]);
});

describe('the workspace delivery loader', () => {
	it('reads the workspace’s delivery as the caller and shapes it', async () => {
		mocks.getWorkspaceDelivery.mockResolvedValue({ boards: [] });
		const out = await run();
		expect(mocks.getWorkspaceDelivery).toHaveBeenCalledWith('organization/acme', 'w1', expect.objectContaining({ accessToken: 'jwt-A' }));
		expect(out.deliveryView).toEqual({ boards: [] });
		expect(out.deliveryError).toBe(null);
		expect(out.tenantKey).toBe('organization/acme');
		expect((out.workspace as { name: string }).name).toBe('API');
	});

	it('a failed read is carried as an error, never as no boards', async () => {
		mocks.getWorkspaceDelivery.mockRejectedValue(new Error('request failed (403)'));
		const out = await run();
		expect(out.deliveryView).toBe(null);
		expect(out.deliveryError).toBe('request failed (403)');
	});

	it('not a member of the org redirects to /you', async () => {
		await expect(run([])).rejects.toMatchObject({ status: 307, location: '/you' });
	});

	it('an unknown workspace redirects to the org workspace list', async () => {
		mocks.listWorkspaces.mockResolvedValue([]);
		await expect(run()).rejects.toMatchObject({ status: 307, location: '/org/acme/workspaces' });
	});
});
