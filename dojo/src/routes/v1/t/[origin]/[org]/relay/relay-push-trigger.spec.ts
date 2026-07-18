// Route-level tests for the relay P4.4 push TRIGGER wiring on:
//   POST relay/inbox   → pushes on agent_to_human approval/decision/stall only
//   POST relay/session → pushes on the transition INTO status=crashed only
//
// Verifies WHICH events push, that the send is fire-and-forget via
// platform.context.waitUntil (never awaited into the response), and that a raise
// still succeeds when platform has no waitUntil (local dev). The send itself is
// mocked — its own logic is covered by relay-push-send.spec.ts. No live Worker/DB.
import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { SendArgs, SendResult } from '$lib/server/relay-push-send';

// The inbox/session routes read one row (session lookup / prior status) then
// insert/upsert. A tiny chainable stub returns queued terminal results.
type Terminal = { data: unknown; error: unknown };
function makeDb() {
	let queue: Terminal[] = [];
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.insert = () => b;
	b.upsert = () => b;
	b.update = () => b;
	b.eq = () => b;
	b.maybeSingle = () => Promise.resolve(queue.shift() ?? { data: null, error: null });
	b.single = () => Promise.resolve(queue.shift() ?? { data: null, error: null });
	return {
		builder: b,
		setQueue(...r: Terminal[]) {
			queue = r;
		}
	};
}
const db = makeDb();

const caller = { userId: 'user-1', tenantId: 't1', membershipId: 'mem-1', role: 'contributor', access: 1 };

// The mocked send — asserted on, never actually sends.
const sendMock = vi.fn(
	async (_args: SendArgs): Promise<SendResult> => ({ gated: false, attempted: 1, delivered: 1, disabled: [] })
);

vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => db.builder,
	ACCESS: { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 }
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveApiKeyAccess: vi.fn(async () => caller),
	resolveTenantAccess: vi.fn(async () => caller),
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 }
}));
vi.mock('$lib/server/relay-push-env', () => ({
	sendRelayPushFromEnv: (args: SendArgs) => sendMock(args),
	vapidFromEnv: () => null
}));

const { POST: inboxPOST } = await import('./inbox/+server');
const { POST: sessionPOST } = await import('./session/+server');

// A platform whose waitUntil records the promises handed to it (fire-and-forget).
function makePlatform() {
	const scheduled: Promise<unknown>[] = [];
	return { platform: { context: { waitUntil: (p: Promise<unknown>) => scheduled.push(p) } }, scheduled };
}

function ev(body: unknown, platform?: unknown) {
	return {
		params: { origin: 'personal', org: 'jerry' },
		request: new Request('http://x/', { method: 'POST', body: JSON.stringify(body) }),
		locals: {},
		platform
	} as never;
}

beforeEach(() => {
	sendMock.mockClear();
});

describe('relay/inbox POST — push trigger', () => {
	for (const kind of ['approval', 'decision', 'stall']) {
		it(`pushes on an agent_to_human ${kind} (via waitUntil)`, async () => {
			db.setQueue({ data: { id: 'sess-1', title: 'Round-trip' }, error: null }, { data: { id: 'ib', seq: 5 }, error: null });
			const { platform, scheduled } = makePlatform();
			const res = await inboxPOST(ev({ run_id: 'run-1', kind }, platform));
			expect(res.status).toBe(200);
			expect(sendMock).toHaveBeenCalledTimes(1);
			const arg = sendMock.mock.calls[0][0];
			expect(arg).toMatchObject({
				userId: 'user-1',
				tenantId: 't1',
				runId: 'run-1',
				runTitle: 'Round-trip',
				signal: { type: 'inbox', kind }
			});
			expect(scheduled).toHaveLength(1); // fire-and-forget, scheduled on waitUntil
		});
	}

	for (const kind of ['chat', 'nudge']) {
		it(`does NOT push a ${kind} row`, async () => {
			db.setQueue({ data: { id: 'sess-1', title: 'R' }, error: null }, { data: { id: 'ib', seq: 5 }, error: null });
			const { platform, scheduled } = makePlatform();
			const res = await inboxPOST(ev({ run_id: 'run-1', kind }, platform));
			expect(res.status).toBe(200);
			expect(sendMock).not.toHaveBeenCalled();
			expect(scheduled).toHaveLength(0);
		});
	}

	it('does NOT push a human_to_agent approval (wrong direction)', async () => {
		db.setQueue({ data: { id: 'sess-1', title: 'R' }, error: null }, { data: { id: 'ib', seq: 5 }, error: null });
		const { platform } = makePlatform();
		const res = await inboxPOST(ev({ run_id: 'run-1', kind: 'approval', direction: 'human_to_agent' }, platform));
		expect(res.status).toBe(200);
		expect(sendMock).not.toHaveBeenCalled();
	});

	it('still succeeds (and fires) when platform has no waitUntil (local dev)', async () => {
		db.setQueue({ data: { id: 'sess-1', title: 'R' }, error: null }, { data: { id: 'ib', seq: 5 }, error: null });
		const res = await inboxPOST(ev({ run_id: 'run-1', kind: 'approval' }, undefined));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ id: 'ib', seq: 5 });
		expect(sendMock).toHaveBeenCalledTimes(1);
	});

	it('does not push when the insert fails (500) — no send on a failed raise', async () => {
		db.setQueue(
			{ data: { id: 'sess-1', title: 'R' }, error: null },
			{ data: null, error: { message: 'insert failed' } }
		);
		const { platform } = makePlatform();
		const res = await inboxPOST(ev({ run_id: 'run-1', kind: 'approval' }, platform));
		expect(res.status).toBe(500);
		expect(sendMock).not.toHaveBeenCalled();
	});
});

describe('relay/session POST — crash push trigger', () => {
	it('pushes crashed on the transition into crashed (prior=running)', async () => {
		db.setQueue({ data: { status: 'running' }, error: null }, { data: { id: 'sess-1' }, error: null });
		const { platform, scheduled } = makePlatform();
		const res = await sessionPOST(ev({ run_id: 'run-1', title: 'R', status: 'crashed' }, platform));
		expect(res.status).toBe(200);
		expect(sendMock).toHaveBeenCalledTimes(1);
		expect(sendMock.mock.calls[0][0]).toMatchObject({ runId: 'run-1', signal: { type: 'crashed' } });
		expect(scheduled).toHaveLength(1);
	});

	it('does NOT re-push when already crashed (prior=crashed)', async () => {
		db.setQueue({ data: { status: 'crashed' }, error: null }, { data: { id: 'sess-1' }, error: null });
		const { platform } = makePlatform();
		const res = await sessionPOST(ev({ run_id: 'run-1', title: 'R', status: 'crashed' }, platform));
		expect(res.status).toBe(200);
		expect(sendMock).not.toHaveBeenCalled();
	});

	it('does NOT push a non-crashed status update', async () => {
		db.setQueue({ data: { status: 'running' }, error: null }, { data: { id: 'sess-1' }, error: null });
		const { platform } = makePlatform();
		const res = await sessionPOST(ev({ run_id: 'run-1', title: 'R', status: 'running' }, platform));
		expect(res.status).toBe(200);
		expect(sendMock).not.toHaveBeenCalled();
	});
});
