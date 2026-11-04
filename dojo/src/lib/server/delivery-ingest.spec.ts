// A board's facts as the daemon sends them (tracker-delivery-metrics D3/D4, #352).
//
// What must never happen here: a body with extra ticket keys (a description, an
// assignee, a URL) let through; a refusal of the database softened into a
// success; an answer other than accepted/stale passed on as one.
import { describe, it, expect } from 'vitest';
import {
	DELIVERY_MAX_TICKETS,
	checkDeliveryBody,
	fileDelivery,
	type DeliveryBody
} from './delivery-ingest';
import type { DojoClient } from './admin-data';

const sprint = (over: Record<string, unknown> = {}) => ({
	external_id: '101',
	seq: 101,
	name: 'Sprint 1',
	state: 'closed',
	start_at: '2026-10-01T00:00:00Z',
	end_at: '2026-10-14T00:00:00Z',
	complete_at: null,
	removed: false,
	sdlc: { velocity: 21 },
	...over
});

const ticket = (over: Record<string, unknown> = {}) => ({
	key: 'AUTH-2',
	title: 'Login',
	status_category: 'done',
	points: 3,
	sprints: ['101'],
	issue_created_at: null,
	...over
});

const body = (over: Record<string, unknown> = {}, facts: Record<string, unknown> = {}) => ({
	v: 1,
	tenant_id: 'b0000000-0000-4000-8000-00000000052a',
	repo_keys: ['github.com/acme/api'],
	board_key: 'ab'.repeat(32),
	kind: 'jira',
	board_name: 'Mobile Scrum',
	facts: {
		history_at: '2026-11-01T10:00:00Z',
		pending_tickets: 0,
		predictability: 12.5,
		predictability_reason: null,
		sprints: [sprint()],
		tickets: [ticket()],
		...facts
	},
	effort: null,
	...over
});

const refused = (r: ReturnType<typeof checkDeliveryBody>) => (r.ok ? null : r);

describe('checkDeliveryBody', () => {
	it('accepts the D3 shape, and a removed sprint with no figures', () => {
		expect(checkDeliveryBody(body()).ok).toBe(true);
		expect(checkDeliveryBody(body({}, { sprints: [sprint({ removed: true, sdlc: null })] })).ok).toBe(true);
		expect(checkDeliveryBody(body({ effort: undefined })).ok).toBe(true);
	});

	it('refuses a ticket carrying anything but the D3 keys (B9): 400', () => {
		for (const extra of [{ description: 'x' }, { assignee: 'ola' }, { url: 'https://acme.atlassian.net/browse/AUTH-2' }]) {
			const r = refused(checkDeliveryBody(body({}, { tickets: [{ ...ticket(), ...extra }] })));
			expect(r?.status, JSON.stringify(extra)).toBe(400);
		}
		expect(refused(checkDeliveryBody(body({}, { tickets: [{ key: 'A-1' }] })))?.status).toBe(400);
	});

	it('refuses a sprint with other keys or a wrong state: 400', () => {
		expect(refused(checkDeliveryBody(body({}, { sprints: [{ ...sprint(), goal: 'x' }] })))?.status).toBe(400);
		expect(refused(checkDeliveryBody(body({}, { sprints: [sprint({ state: 'done' })] })))?.status).toBe(400);
		expect(refused(checkDeliveryBody(body({}, { sprints: [sprint({ removed: undefined })] })))?.status).toBe(400);
	});

	it('refuses an unknown v, a bad tenant, board key or repo list: 400', () => {
		for (const over of [
			{ v: 2 },
			{ tenant_id: 'acme' },
			{ board_key: 'AB'.repeat(32) },
			{ board_key: 'ab' },
			{ repo_keys: [] },
			{ repo_keys: [''] },
			{ kind: '' },
			{ board_name: 7 },
			{ facts: null }
		]) {
			expect(refused(checkDeliveryBody(body(over)))?.status, JSON.stringify(over)).toBe(400);
		}
		expect(refused(checkDeliveryBody(null))?.status).toBe(400);
		expect(refused(checkDeliveryBody(body({}, { pending_tickets: -1 })))?.status).toBe(400);
		expect(refused(checkDeliveryBody(body({}, { pending_tickets: 1.5 })))?.status).toBe(400);
	});

	it('refuses an effort part until it is accepted (F3): 400', () => {
		expect(refused(checkDeliveryBody(body({ effort: { tickets: [] } })))?.status).toBe(400);
	});

	it('refuses more than 5,000 tickets: 413', () => {
		const many = Array.from({ length: DELIVERY_MAX_TICKETS + 1 }, (_, i) => ticket({ key: `A-${i}` }));
		expect(refused(checkDeliveryBody(body({}, { tickets: many })))?.status).toBe(413);
	});
});

function db(answer: { data: unknown; error: { code?: string; message: string } | null }) {
	const calls: { fn: string; args: Record<string, unknown> }[] = [];
	const client = {
		rpc: (fn: string, args: Record<string, unknown>) => {
			calls.push({ fn, args });
			return Promise.resolve(answer);
		}
	} as unknown as DojoClient;
	return { client, calls };
}

describe('fileDelivery', () => {
	const ok = checkDeliveryBody(body());
	const b = (ok.ok ? ok.body : null) as DeliveryBody;

	it('files the board as one call, never sending the tenant inside the board', async () => {
		const { client, calls } = db({ data: 'accepted', error: null });
		expect(await fileDelivery(client, b)).toEqual({ facts: 'accepted' });
		expect(calls).toHaveLength(1);
		expect(calls[0].fn).toBe('file_delivery_facts');
		expect(calls[0].args.p_tenant).toBe(b.tenant_id);
		expect(calls[0].args.p_repo_keys).toEqual(b.repo_keys);
		expect(calls[0].args.p_board).toEqual({
			board_key: b.board_key,
			kind: 'jira',
			board_name: 'Mobile Scrum',
			facts: b.facts
		});
	});

	it('passes a lesser copy on as stale', async () => {
		const { client } = db({ data: 'stale', error: null });
		expect(await fileDelivery(client, b)).toEqual({ facts: 'stale' });
	});

	it('keeps each refusal its status: 403, 404, 409; a failure is 500', async () => {
		for (const [code, status] of [
			['42501', 403],
			['P0002', 404],
			['55000', 409],
			['08006', 500]
		] as const) {
			const { client } = db({ data: null, error: { code, message: 'no' } });
			await expect(fileDelivery(client, b)).rejects.toMatchObject({ status });
		}
	});

	it('an answer that is neither accepted nor stale is a failure, not a success', async () => {
		const { client } = db({ data: null, error: null });
		await expect(fileDelivery(client, b)).rejects.toMatchObject({ status: 500 });
	});
});
