// The dōjō page's delivery reads (tracker-delivery-metrics D5; #352): the right
// tenant-addressed URL, the caller's token, and a failure that throws.
import { describe, it, expect, vi } from 'vitest';
import { getWorkspaceDelivery, getWorkspaceDeliveryTickets } from './workspace-delivery-data';

function fetchOf(status: number, body: unknown) {
	return vi.fn(async (_url: string, _init?: RequestInit) => new Response(JSON.stringify(body), { status }));
}

/** A mock as the client's `fetch` option; the mock keeps its calls for assertions. */
const asFetch = (f: ReturnType<typeof fetchOf>) => f as unknown as typeof fetch;

describe('getWorkspaceDelivery', () => {
	it('reads …/workspaces/{id}/delivery for the tenant as the caller', async () => {
		const f = fetchOf(200, { boards: [] });
		expect(await getWorkspaceDelivery('organization/acme', 'w-1', { fetch: asFetch(f), accessToken: 'jwt' })).toEqual({ boards: [] });
		const [url, init] = f.mock.calls[0];
		expect(url).toMatch(/\/v1\/t\/organization\/acme\/workspaces\/w-1\/delivery$/);
		expect((init!.headers as Record<string, string>).Authorization).toBe('Bearer jwt');
	});

	it('a failed read throws, so the page can say so', async () => {
		await expect(getWorkspaceDelivery('organization/acme', 'w-1', { fetch: asFetch(fetchOf(403, { error: 'no' })) })).rejects.toThrow();
	});
});

describe('getWorkspaceDeliveryTickets', () => {
	it('reads one sprint’s tickets', async () => {
		const f = fetchOf(200, { tickets: [{ key: 'AUTH-2', title: 'Two' }] });
		const out = await getWorkspaceDeliveryTickets('organization/acme', 'w-1', 'b 1', { sprint: '10/2' }, { fetch: asFetch(f) });
		expect(out).toEqual([{ key: 'AUTH-2', title: 'Two' }]);
		expect(f.mock.calls[0][0]).toMatch(/\/workspaces\/w-1\/delivery\/tickets\?board=b%201&sprint=10%2F2$/);
	});

	it('reads the backlog', async () => {
		const f = fetchOf(200, { tickets: [] });
		await getWorkspaceDeliveryTickets('organization/acme', 'w-1', 'b1', { backlog: true }, { fetch: asFetch(f) });
		expect(f.mock.calls[0][0]).toMatch(/delivery\/tickets\?board=b1&backlog=1$/);
	});

	it('a failed read throws', async () => {
		await expect(
			getWorkspaceDeliveryTickets('organization/acme', 'w-1', 'b1', {}, { fetch: asFetch(fetchOf(500, { error: 'x' })) })
		).rejects.toThrow();
	});
});
