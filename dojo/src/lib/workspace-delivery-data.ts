// A workspace's delivery facts — its tracker boards, their sprints and tickets —
// as the dōjō page reads them (spec docs/spec/daemon/tracker-delivery-metrics.md
// D5; #352): the wire behind GET /v1/t/{origin}/{org}/workspaces/{id}/delivery and
// …/delivery/tickets, plus the fetch. Read as the caller; a failure throws, so the
// page shows an error, never an empty board list.
import { authHeaders, dojoApiUrl, encodeTenant, parseJson, type DojoCallOpts } from './dojo-api';
import type { DeliveryTicketWire, WorkspaceDeliveryWire } from './delivery-wire';

export type {
	DeliveryBoardWire,
	DeliverySdlc,
	DeliverySprintWire,
	DeliveryTicketWire,
	WorkspaceDeliveryWire
} from './delivery-wire';

const base = (tenantKey: string, workspaceId: string) =>
	`${dojoApiUrl}/v1/t/${encodeTenant(tenantKey)}/workspaces/${encodeURIComponent(workspaceId)}/delivery`;

/** `GET …/workspaces/{id}/delivery` — the workspace's boards, each with every sprint. */
export async function getWorkspaceDelivery(
	tenantKey: string,
	workspaceId: string,
	opts: DojoCallOpts = {}
): Promise<WorkspaceDeliveryWire> {
	const f = opts.fetch ?? fetch;
	const res = await f(base(tenantKey, workspaceId), { headers: authHeaders(opts.accessToken) });
	return parseJson<WorkspaceDeliveryWire>(res);
}

/** `GET …/workspaces/{id}/delivery/tickets` — one sprint's, the backlog's, or all of a board's tickets. */
export async function getWorkspaceDeliveryTickets(
	tenantKey: string,
	workspaceId: string,
	boardId: string,
	which: { sprint?: string; backlog?: boolean },
	opts: DojoCallOpts = {}
): Promise<DeliveryTicketWire[]> {
	const q = new URLSearchParams({ board: boardId });
	if (which.sprint !== undefined) q.set('sprint', which.sprint);
	if (which.backlog) q.set('backlog', '1');
	const f = opts.fetch ?? fetch;
	const res = await f(`${base(tenantKey, workspaceId)}/tickets?${q.toString().replace(/\+/g, '%20')}`, {
		headers: authHeaders(opts.accessToken)
	});
	return (await parseJson<{ tickets: DeliveryTicketWire[] }>(res)).tickets;
}
