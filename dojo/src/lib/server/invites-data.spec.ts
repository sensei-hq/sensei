import { describe, it, expect } from 'vitest';
import { parseNewInvite, createInvite, acceptInvite, AdminError, type DojoClient } from './invites-data';

// A chainable stub with an ordered result queue: each terminal (`.single()` /
// `.maybeSingle()` / `.is()`) shifts the next result, in call order. Captures the
// insert/update payloads + the table each ran against.
function makeDb(...results: { data?: unknown; error: unknown }[]) {
	const queue = [...results];
	const shift = () => queue.shift() ?? { data: null, error: null };
	const captured: {
		table?: string;
		inserts: { table?: string; payload: Record<string, unknown> }[];
		updates: { table?: string; payload: Record<string, unknown> }[];
		eqs: [string, unknown][];
	} = { inserts: [], updates: [], eqs: [] };
	const b: Record<string, unknown> = {};
	b.from = (t: string) => {
		captured.table = t;
		return b;
	};
	b.insert = (p: Record<string, unknown>) => {
		captured.inserts.push({ table: captured.table, payload: p });
		return b;
	};
	b.update = (p: Record<string, unknown>) => {
		captured.updates.push({ table: captured.table, payload: p });
		return b;
	};
	b.select = () => b;
	b.eq = (c: string, v: unknown) => {
		captured.eqs.push([c, v]);
		return b;
	};
	b.single = () => Promise.resolve(shift());
	b.maybeSingle = () => Promise.resolve(shift());
	b.is = () => Promise.resolve(shift());
	return { db: b as unknown as DojoClient, captured };
}

const NOW = Date.parse('2026-08-03T12:00:00Z');
const FUTURE = '2026-08-10T12:00:00Z';
const PAST = '2026-08-01T12:00:00Z';

describe('parseNewInvite', () => {
	it('lowercases the email, defaults role to contributor, requires a valid kind', () => {
		expect(parseNewInvite({ email: 'Ada@Acme.CO', kind: 'client' })).toEqual({
			email: 'ada@acme.co',
			role: 'contributor',
			kind: 'client'
		});
		expect(parseNewInvite({ email: 'a@b.co', role: 'lead', kind: 'employer' }).role).toBe('lead');
	});
	it('rejects a garbage email or bad kind (400)', () => {
		expect(() => parseNewInvite({ email: 'nope', kind: 'client' })).toThrow();
		expect(() => parseNewInvite({ email: 'a@b.co', kind: 'wizard' })).toThrow();
	});
});

describe('createInvite', () => {
	it('inserts an invite with a token + a +7d expiry and returns the row', async () => {
		const { db, captured } = makeDb({
			data: { id: 'inv1', token: 'tok', email: 'a@b.co', role: 'contributor', expires_at: FUTURE },
			error: null
		});
		const out = await createInvite(db, 't1', 'admin-uid', { email: 'a@b.co', role: 'contributor', kind: 'client' }, NOW);
		expect(out.id).toBe('inv1');
		const payload = captured.inserts[0].payload;
		expect(captured.inserts[0].table).toBe('invites');
		expect(payload).toMatchObject({ tenant_id: 't1', email: 'a@b.co', role: 'contributor', kind: 'client', invited_by: 'admin-uid' });
		expect(typeof payload.token).toBe('string');
		expect((payload.token as string).length).toBeGreaterThan(20); // crypto.randomUUID
		expect(Date.parse(payload.expires_at as string)).toBe(NOW + 7 * 24 * 60 * 60 * 1000);
	});
});

describe('acceptInvite — every gate fails closed, none provisions unless all pass', () => {
	const invite = (over: Record<string, unknown> = {}) => ({
		data: {
			id: 'inv1',
			tenant_id: 't1',
			email: 'ada@acme.co',
			role: 'contributor',
			kind: 'client',
			expires_at: FUTURE,
			accepted_at: null,
			...over
		},
		error: null
	});

	it('404 when the token is unknown', async () => {
		const { db } = makeDb({ data: null, error: null });
		await expect(acceptInvite(db, 'u1', 'ada@acme.co', 'tok', NOW)).rejects.toMatchObject({ status: 404 });
	});
	it('400 when the token is empty (no read)', async () => {
		const { db } = makeDb();
		await expect(acceptInvite(db, 'u1', 'ada@acme.co', '', NOW)).rejects.toMatchObject({ status: 400 });
	});
	it('409 when already accepted (single-use)', async () => {
		const { db } = makeDb(invite({ accepted_at: PAST }));
		await expect(acceptInvite(db, 'u1', 'ada@acme.co', 'tok', NOW)).rejects.toMatchObject({ status: 409 });
	});
	it('410 when expired', async () => {
		const { db } = makeDb(invite({ expires_at: PAST }));
		await expect(acceptInvite(db, 'u1', 'ada@acme.co', 'tok', NOW)).rejects.toMatchObject({ status: 410 });
	});
	it('403 when the caller email does not match the invite (the real gate)', async () => {
		const { db } = makeDb(invite());
		await expect(acceptInvite(db, 'u1', 'mallory@evil.co', 'tok', NOW)).rejects.toMatchObject({ status: 403 });
	});
	it('403 when the caller has no verified email', async () => {
		const { db } = makeDb(invite());
		await expect(acceptInvite(db, 'u1', null, 'tok', NOW)).rejects.toMatchObject({ status: 403 });
	});
	it('provisions the membership at the invited role + stamps accepted on a match (case-insensitive)', async () => {
		const { db, captured } = makeDb(
			invite(), // loadInvite
			{ data: { id: 'm1', role: 'contributor' }, error: null }, // addMember
			{ error: null } // accepted stamp
		);
		const out = await acceptInvite(db, 'u1', 'ADA@acme.co', 'tok', NOW);
		expect(out).toEqual({ tenant_id: 't1', role: 'contributor' });
		// membership provisioned at the invited role for the caller
		expect(captured.inserts[0]).toMatchObject({
			table: 'memberships',
			payload: { tenant_id: 't1', user_id: 'u1', role: 'contributor', kind: 'client' }
		});
		// single-use stamp, guarded on accepted_at IS NULL
		expect(captured.updates[0]).toMatchObject({ table: 'invites' });
		expect(captured.updates[0].payload.accepted_at).toBeTruthy();
	});
});
