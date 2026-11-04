// A workspace's delivery facts — its tracker boards, their sprints and tickets —
// read as the caller (spec docs/spec/daemon/tracker-delivery-metrics.md D4, D5; #352).
//
// The order every read keeps: can the caller read the workspace (403 before any
// read, as getWorkspaceMetrics), is it this dōjō's (404), then the reads. The
// database functions gate every row on the reader too; this is the status a
// person sees. A failed read is an error, never an empty board list (D83).
import { AdminError, type DojoClient } from './admin-data';
import { refusalStatus } from './db-refusal';
import { readAllPages } from './repositories';
import { callFn, requireInTenant } from './tenant-rows';

export interface DeliverySprintRow {
	external_id: string;
	seq: number;
	name: string;
	state: 'future' | 'active' | 'closed';
	start_at: string | null;
	end_at: string | null;
	complete_at: string | null;
	sdlc: Record<string, unknown> | null;
}

export interface DeliveryBoard {
	board_id: string;
	board_key: string;
	kind: string;
	name: string;
	predictability: number | null;
	predictability_reason: string | null;
	history_at: string | null;
	pending_tickets: number;
	reported_at: string;
	/** Every non-removed sprint, in the daemon's order (never re-sorted here). */
	sprints: DeliverySprintRow[];
}

export interface DeliveryTicketRow {
	key: string;
	title: string | null;
	status_category: string | null;
	points: number | null;
	sprints: string[];
	issue_created_at: string | null;
}

async function requireReader(db: DojoClient, tenantId: string, workspaceId: string): Promise<void> {
	const mayRead = await callFn<boolean>(db, 'can_read_workspace', { p_workspace: workspaceId });
	if (!mayRead) throw new AdminError(403, 'you may not read this workspace’s delivery');
	await requireInTenant(db, 'workspaces', tenantId, workspaceId, 'workspace');
}

/** The workspace's boards, each with every page of its sprints. */
export async function getWorkspaceDelivery(
	db: DojoClient,
	tenantId: string,
	workspaceId: string
): Promise<{ boards: DeliveryBoard[] }> {
	await requireReader(db, tenantId, workspaceId);
	const { data, error } = await db.rpc('workspace_delivery_boards', { p_workspace: workspaceId });
	if (error) throw new AdminError(refusalStatus(error), error.message);
	const boards = (data ?? []) as Omit<DeliveryBoard, 'sprints'>[];
	const withSprints = await Promise.all(
		boards.map(async (b) => ({
			...b,
			sprints: await readAllPages<DeliverySprintRow>((from, to) =>
				db.rpc('workspace_delivery_sprints', {
					p_workspace: workspaceId,
					p_board: b.board_id,
					p_limit: to - from + 1,
					p_offset: from
				})
			)
		}))
	);
	return { boards: withSprints };
}

/** One board's tickets: one sprint's, the backlog's, or all. */
export async function getDeliveryTickets(
	db: DojoClient,
	tenantId: string,
	workspaceId: string,
	boardId: string,
	which: { sprint?: string; backlog?: boolean }
): Promise<DeliveryTicketRow[]> {
	await requireReader(db, tenantId, workspaceId);
	return readAllPages<DeliveryTicketRow>((from, to) =>
		db.rpc('workspace_delivery_tickets', {
			p_workspace: workspaceId,
			p_board: boardId,
			p_sprint: which.sprint ?? null,
			p_backlog: which.backlog === true,
			p_limit: to - from + 1,
			p_offset: from
		})
	);
}
