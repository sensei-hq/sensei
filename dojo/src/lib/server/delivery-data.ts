// A workspace's delivery facts — its tracker boards, their sprints and tickets —
// read as the caller (spec docs/spec/daemon/tracker-delivery-metrics.md D4, D5; #352).
//
// The order every read keeps: can the caller read the workspace (403 before any
// read, as getWorkspaceMetrics), is it this dōjō's (404), then the reads. The
// database functions gate every row on the reader too; this is the status a
// person sees. A failed read, or one that answers no list, is an error — never an
// empty board list (D83). A board's copy is replaced whole by a push, and the
// reads are pages of separate statements, so each read is checked against its
// board's `reported_at` before and after: one that spans a replacement is read
// again, and one that keeps changing is a 409 rather than a mix of two reports.
import { AdminError, type DojoClient } from './admin-data';
import { refusalStatus } from './db-refusal';
import { readAllPages } from './repositories';
import { callFn, requireInTenant } from './tenant-rows';
import type {
	DeliveryBoardWire,
	DeliverySprintWire,
	DeliveryTicketWire,
	WorkspaceDeliveryWire
} from '$lib/delivery-wire';

/** Reads of one workspace's delivery that span a replacement are retried this often. */
const ATTEMPTS = 3;

type BoardRow = Omit<DeliveryBoardWire, 'sprints'>;
type Answer = { data: unknown; error: { code?: string; message: string } | null };

/** A set-returning function answers a list; anything else is a failure, never "none". */
function listOrFail(fn: string, answer: Answer): Answer {
	if (answer.error || Array.isArray(answer.data)) return answer;
	return { data: null, error: { message: `${fn} answered no list` } };
}

async function requireReader(db: DojoClient, tenantId: string, workspaceId: string): Promise<void> {
	const mayRead = await callFn<boolean>(db, 'can_read_workspace', { p_workspace: workspaceId });
	if (!mayRead) throw new AdminError(403, 'you may not read this workspace’s delivery');
	await requireInTenant(db, 'workspaces', tenantId, workspaceId, 'workspace');
}

async function readBoards(db: DojoClient, workspaceId: string): Promise<BoardRow[]> {
	const { data, error } = listOrFail(
		'workspace_delivery_boards',
		await db.rpc('workspace_delivery_boards', { p_workspace: workspaceId })
	);
	if (error) throw new AdminError(refusalStatus(error), error.message);
	return data as BoardRow[];
}

/** Each board's id → the report its copy came from. */
const versions = (boards: BoardRow[]) => JSON.stringify(boards.map((b) => [b.board_id, b.reported_at]));

/** Run `read` until the boards' reports are the same before and after it. */
async function consistent<T>(
	db: DojoClient,
	workspaceId: string,
	read: (boards: BoardRow[]) => Promise<T>
): Promise<T> {
	for (let attempt = 0; attempt < ATTEMPTS; attempt++) {
		const before = await readBoards(db, workspaceId);
		const out = await read(before);
		if (versions(await readBoards(db, workspaceId)) === versions(before)) return out;
	}
	throw new AdminError(409, 'the board kept changing while it was read; try again');
}

/** The workspace's boards, each with every page of its sprints, from one report each. */
export async function getWorkspaceDelivery(
	db: DojoClient,
	tenantId: string,
	workspaceId: string
): Promise<WorkspaceDeliveryWire> {
	await requireReader(db, tenantId, workspaceId);
	const boards = await consistent(db, workspaceId, (rows) =>
		Promise.all(
			rows.map(async (b) => ({
				...b,
				sprints: await readAllPages<DeliverySprintWire>(async (from, to) =>
					listOrFail(
						'workspace_delivery_sprints',
						await db.rpc('workspace_delivery_sprints', {
							p_workspace: workspaceId,
							p_board: b.board_id,
							p_limit: to - from + 1,
							p_offset: from
						})
					)
				)
			}))
		)
	);
	return { boards };
}

/** One board's tickets — one sprint's, the backlog's, or all — from one report. */
export async function getDeliveryTickets(
	db: DojoClient,
	tenantId: string,
	workspaceId: string,
	boardId: string,
	which: { sprint?: string; backlog?: boolean }
): Promise<DeliveryTicketWire[]> {
	await requireReader(db, tenantId, workspaceId);
	return consistent(db, workspaceId, async (boards) => {
		if (!boards.some((b) => b.board_id === boardId)) throw new AdminError(404, 'no such board in this workspace');
		return readAllPages<DeliveryTicketWire>(async (from, to) =>
			listOrFail(
				'workspace_delivery_tickets',
				await db.rpc('workspace_delivery_tickets', {
					p_workspace: workspaceId,
					p_board: boardId,
					p_sprint: which.sprint ?? null,
					p_backlog: which.backlog === true,
					p_limit: to - from + 1,
					p_offset: from
				})
			)
		);
	});
}
