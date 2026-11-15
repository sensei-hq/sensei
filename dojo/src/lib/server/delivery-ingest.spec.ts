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
	withdrawDelivery,
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

/** One effort row (A2); `values` over a ticket's wire fields unless overridden. */
const row = (values: Record<string, unknown> = {}) => ({
	repo_keys: ['github.com/acme/api'],
	values: { sessions: 1, worked_ms: 60000, prompts: null, on_hold_by_reason: { unknown: 3 }, ...values },
	reasons: { prompts: 'not_derived' }
});

const effort = (over: Record<string, unknown> = {}) => ({
	source: '5a000000-0000-4000-8000-000000000001',
	project: { repo_keys: ['github.com/acme/api'], values: { sessions: 1, tokens: 900, sessions_without_tokens: 0, prompts: 4 }, reasons: {} },
	sprints: [{ repo_keys: ['github.com/acme/api'], values: { sessions: 1, active_secs: 60, active_n: 1, by_source: { branch: 1, prompt: 0 }, first_at: null }, reasons: {}, external_id: '101' }],
	tickets: [{ ...row(), key: 'AUTH-2' }],
	...over
});

describe('checkDeliveryBody', () => {
	it('accepts the D3 shape, and a removed sprint with no figures', () => {
		expect(checkDeliveryBody(body()).ok).toBe(true);
		expect(checkDeliveryBody(body({}, { sprints: [sprint({ removed: true, sdlc: null })] })).ok).toBe(true);
		expect(checkDeliveryBody(body({ effort: undefined })).ok).toBe(true);
	});

	it('takes a sync age of whole seconds from 0 to ten years, or none (A5); else 400', () => {
		for (const age of [undefined, null, 0, 7200, 315360000]) {
			expect(checkDeliveryBody(body({}, { synced_age_s: age })).ok, String(age)).toBe(true);
		}
		for (const age of [-1, 1.5, 1e15, 315360001, '60', NaN]) {
			const r = refused(checkDeliveryBody(body({}, { synced_age_s: age })));
			expect(r?.status, String(age)).toBe(400);
			expect(r?.message).toBe('facts.synced_age_s must be a whole number of seconds from 0 to 315360000, or null');
		}
	});

	it('refuses a history_at that is not a timestamp, before it reaches the cast: 400', () => {
		const r = refused(checkDeliveryBody(body({}, { history_at: 'garbage' })));
		expect(r?.status).toBe(400);
		expect(r?.message).toBe('facts.history_at must be a timestamp or null');
		expect(checkDeliveryBody(body({}, { history_at: null })).ok).toBe(true);
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

	it('accepts the effort part (A2): a project row, sprint rows and ticket rows', () => {
		const r = checkDeliveryBody(body({ effort: effort() }));
		expect(r.ok).toBe(true);
		expect(r.ok && r.body.effort).toEqual(effort());
		expect(checkDeliveryBody(body({ effort: { source: effort().source, project: null, sprints: [], tickets: [] } })).ok).toBe(true);
	});

	it('refuses effort with other keys, a value outside the wire, or a foreign repository: 400', () => {
		const bad = [
			{ ...effort(), extra: [] },
			effort({ tickets: [{ ...row(), key: 'AUTH-2', note: 'x' }] }),
			// The rate and the local calibration never travel (B10, B14).
			effort({ tickets: [{ ...row({ hourly_rate: 90 }), key: 'AUTH-2' }] }),
			effort({ tickets: [{ ...row({ expected_minutes: 30 }), key: 'AUTH-2' }] }),
			effort({ sprints: [{ ...row({ linked_other: 1 }), external_id: '101' }] }),
			effort({ project: row({ worked_ms: 1 }) }),
			effort({ tickets: [{ ...row(), key: 'AUTH-2', repo_keys: ['github.com/acme/other'] }] }),
			effort({ tickets: [{ ...row(), key: 'AUTH-2', repo_keys: [] }] }),
			effort({ tickets: [{ ...row(), key: 'AUTH-2', reasons: { prompts: 7 } }] }),
			effort({ tickets: [{ ...row(), key: '' }] }),
			effort({ tickets: [{ ...row(), key: 'AUTH-2' }, { ...row(), key: 'AUTH-2' }] }),
			// A value every reader's merge would fail on, or a row with no session (a decoy).
			effort({ tickets: [{ ...row({ work_started_at: 'zzz' }), key: 'AUTH-2' }] }),
			effort({ tickets: [{ ...row({ submitted_via: 'email' }), key: 'AUTH-2' }] }),
			effort({ tickets: [{ ...row({ worked_ms: Number.NaN }), key: 'AUTH-2' }] }),
			effort({ tickets: [{ ...row({ sessions: 0 }), key: 'AUTH-2' }] }),
			effort({ tickets: [{ ...row({ on_hold_by_reason: { unknown: 'x' } }), key: 'AUTH-2' }] }),
			effort({ sprints: [{ ...row(), external_id: '101', values: { sessions: 1, by_source: [1] } }] }),
			effort({ project: { ...row(), values: { sessions: 1, tokens: 9 }, repo_keys: ['github.com/acme/other'] } }),
			// The install that pushed is named, as a uuid.
			effort({ source: undefined }),
			effort({ source: 'laptop' })
		];
		for (const e of bad) {
			expect(refused(checkDeliveryBody(body({ effort: e })))?.status, JSON.stringify(e)).toBe(400);
		}
	});

	it('refuses more than 1,000 effort sprints: 413', () => {
		const many = Array.from({ length: 1001 }, (_, i) => ({ ...row(), values: { sessions: 1 }, external_id: `${i}` }));
		expect(refused(checkDeliveryBody(body({ effort: effort({ sprints: many }) })))?.status).toBe(413);
	});

	it('refuses more than 5,000 effort tickets: 413', () => {
		const many = Array.from({ length: DELIVERY_MAX_TICKETS + 1 }, (_, i) => ({ ...row(), key: `A-${i}` }));
		expect(refused(checkDeliveryBody(body({ effort: effort({ tickets: many }) })))?.status).toBe(413);
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
		const { client, calls } = db({ data: { facts: 'accepted' }, error: null });
		expect(await fileDelivery(client, b)).toEqual({ facts: 'accepted' });
		expect(calls).toHaveLength(1);
		expect(calls[0].fn).toBe('file_delivery');
		expect(calls[0].args.p_effort).toBeNull();
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
		const { client } = db({ data: { facts: 'stale' }, error: null });
		expect(await fileDelivery(client, b)).toEqual({ facts: 'stale' });
	});

	it('files both parts in one call and answers both outcomes', async () => {
		const withEffort = checkDeliveryBody(body({ effort: effort() }));
		const e = (withEffort.ok ? withEffort.body : null) as DeliveryBody;
		const { client, calls } = db({ data: { facts: 'stale', effort: 'accepted' }, error: null });
		expect(await fileDelivery(client, e)).toEqual({ facts: 'stale', effort: 'accepted' });
		expect(calls).toHaveLength(1);
		expect(calls[0].args.p_effort).toEqual(effort());
		const { client: odd } = db({ data: { facts: 'accepted', effort: 'maybe' }, error: null });
		await expect(fileDelivery(odd, e)).rejects.toMatchObject({ status: 500 });
		const { client: missing } = db({ data: { facts: 'accepted' }, error: null });
		await expect(fileDelivery(missing, e)).rejects.toMatchObject({ status: 500 });
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
		for (const data of [null, 'accepted', { facts: 'maybe' }]) {
			const { client } = db({ data, error: null });
			await expect(fileDelivery(client, b), JSON.stringify(data)).rejects.toMatchObject({ status: 500 });
		}
	});
});

describe('withdrawDelivery', () => {
	const key = 'ab'.repeat(32);
	const tenant = 'b0000000-0000-4000-8000-00000000052a';
	const source = '5a000000-0000-4000-8000-000000000001';

	it('withdraws as the caller, naming the install; answers whether anything went', async () => {
		const { client, calls } = db({ data: true, error: null });
		expect(await withdrawDelivery(client, tenant, key, source)).toBe(true);
		expect(calls).toEqual([{ fn: 'withdraw_delivery', args: { p_tenant: tenant, p_board_key: key, p_source: source } }]);
		const { client: none } = db({ data: false, error: null });
		expect(await withdrawDelivery(none, tenant, key, source)).toBe(false);
	});

	it('an answer that is not a yes or no is a failure', async () => {
		const { client } = db({ data: null, error: null });
		await expect(withdrawDelivery(client, tenant, key, source)).rejects.toMatchObject({ status: 500 });
	});
});
