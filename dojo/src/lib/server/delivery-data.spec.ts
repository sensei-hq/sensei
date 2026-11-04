// A workspace's delivery reads (tracker-delivery-metrics D4, #352).
//
// What must never happen here: a non-reader answered (403 first, before any read);
// a workspace of another dōjō reached through this one's URL (404); a failed read
// shown as "no boards"; a board's sprints cut at one page.
import { describe, it, expect } from 'vitest';
import { getDeliveryTickets, getWorkspaceDelivery } from './delivery-data';
import type { DojoClient } from './admin-data';

const W = 'c0000000-0000-4000-8000-000000000521';
const T = 'b0000000-0000-4000-8000-00000000052a';
const B = 'f2000000-0000-4000-8000-000000000001';

type Answer = { data: unknown; error: { code?: string; message: string } | null };

function makeDb(opts: {
	canRead?: boolean;
	inTenant?: boolean;
	boards?: Answer;
	sprintPages?: unknown[][];
	sprintsError?: Answer['error'];
	tickets?: Answer;
}) {
	const calls: Array<{ fn: string; args: Record<string, unknown> }> = [];
	const rpc = (fn: string, args: Record<string, unknown>) => {
		calls.push({ fn, args });
		if (fn === 'can_read_workspace') return Promise.resolve({ data: opts.canRead ?? true, error: null });
		if (fn === 'workspace_delivery_boards') return Promise.resolve(opts.boards ?? { data: [], error: null });
		if (fn === 'workspace_delivery_sprints') {
			if (opts.sprintsError) return Promise.resolve({ data: null, error: opts.sprintsError });
			const page = Math.floor((args.p_offset as number) / (args.p_limit as number));
			return Promise.resolve({ data: opts.sprintPages?.[page] ?? [], error: null });
		}
		if (fn === 'workspace_delivery_tickets') return Promise.resolve(opts.tickets ?? { data: [], error: null });
		throw new Error(`unexpected rpc ${fn}`);
	};
	const from = (table: string) => {
		calls.push({ fn: `from:${table}`, args: {} });
		const chain: Record<string, unknown> = {};
		chain.select = () => chain;
		chain.eq = () => chain;
		chain.maybeSingle = () => Promise.resolve({ data: opts.inTenant === false ? null : { id: W }, error: null });
		return chain;
	};
	return { db: { rpc, from } as unknown as DojoClient, calls };
}

const board = {
	board_id: B,
	board_key: 'ab'.repeat(32),
	kind: 'jira',
	name: 'Mobile Scrum',
	predictability: 12.5,
	predictability_reason: null,
	history_at: '2026-11-01T10:00:00Z',
	pending_tickets: 0,
	reported_at: '2026-11-04T10:00:00Z'
};
const sprintRow = (id: string) => ({
	external_id: id,
	seq: Number(id),
	name: `Sprint ${id}`,
	state: 'closed',
	start_at: null,
	end_at: null,
	complete_at: null,
	sdlc: null
});

describe('getWorkspaceDelivery', () => {
	it('a caller who may not read the workspace is 403 before anything is read', async () => {
		const { db, calls } = makeDb({ canRead: false });
		await expect(getWorkspaceDelivery(db, T, W)).rejects.toMatchObject({ status: 403 });
		expect(calls.map((c) => c.fn)).toEqual(['can_read_workspace']);
	});

	it('a workspace of another dōjō is 404', async () => {
		const { db } = makeDb({ inTenant: false });
		await expect(getWorkspaceDelivery(db, T, W)).rejects.toMatchObject({ status: 404 });
	});

	it('answers each board with every page of its sprints, in the order read', async () => {
		const first = Array.from({ length: 1000 }, (_, i) => sprintRow(String(2000 - i)));
		const { db, calls } = makeDb({
			boards: { data: [board], error: null },
			sprintPages: [first, [sprintRow('7')]]
		});
		const out = await getWorkspaceDelivery(db, T, W);
		expect(out.boards).toHaveLength(1);
		expect(out.boards[0].name).toBe('Mobile Scrum');
		expect(out.boards[0].sprints).toHaveLength(1001);
		expect(out.boards[0].sprints.at(-1)!.external_id).toBe('7');
		const sprintCalls = calls.filter((c) => c.fn === 'workspace_delivery_sprints');
		expect(sprintCalls[0].args).toMatchObject({ p_workspace: W, p_board: B, p_offset: 0 });
	});

	it('no boards is an honest empty list', async () => {
		const { db } = makeDb({ boards: { data: [], error: null } });
		expect(await getWorkspaceDelivery(db, T, W)).toEqual({ boards: [] });
	});

	it('an answer that is not a list is an error, never "no boards"', async () => {
		const { db } = makeDb({ boards: { data: null, error: null } });
		await expect(getWorkspaceDelivery(db, T, W)).rejects.toMatchObject({ status: 500 });
	});

	it('a failed read is an error, never "no boards"', async () => {
		const failedBoards = makeDb({ boards: { data: null, error: { message: 'reset' } } });
		await expect(getWorkspaceDelivery(failedBoards.db, T, W)).rejects.toMatchObject({ status: 500 });
		const failedSprints = makeDb({ boards: { data: [board], error: null }, sprintsError: { message: 'reset' } });
		await expect(getWorkspaceDelivery(failedSprints.db, T, W)).rejects.toMatchObject({ status: 500 });
	});
});

describe('getDeliveryTickets', () => {
	it('reads one sprint’s tickets, gated like the boards', async () => {
		const { db, calls } = makeDb({ tickets: { data: [{ key: 'AUTH-2', title: 'Two' }], error: null } });
		const out = await getDeliveryTickets(db, T, W, B, { sprint: '101' });
		expect(out).toEqual([{ key: 'AUTH-2', title: 'Two' }]);
		expect(calls[0].fn).toBe('can_read_workspace');
		expect(calls.find((c) => c.fn === 'workspace_delivery_tickets')!.args).toMatchObject({
			p_workspace: W,
			p_board: B,
			p_sprint: '101',
			p_backlog: false
		});
	});

	it('reads the backlog', async () => {
		const { db, calls } = makeDb({});
		await getDeliveryTickets(db, T, W, B, { backlog: true });
		expect(calls.find((c) => c.fn === 'workspace_delivery_tickets')!.args).toMatchObject({ p_sprint: null, p_backlog: true });
	});

	it('a non-reader is 403; a failed read is 500', async () => {
		await expect(getDeliveryTickets(makeDb({ canRead: false }).db, T, W, B, {})).rejects.toMatchObject({ status: 403 });
		const failed = makeDb({ tickets: { data: null, error: { message: 'reset' } } });
		await expect(getDeliveryTickets(failed.db, T, W, B, {})).rejects.toMatchObject({ status: 500 });
	});
});
