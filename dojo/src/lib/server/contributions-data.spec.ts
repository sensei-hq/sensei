import { describe, it, expect } from 'vitest';
import {
	userMembershipIds,
	listUserContributions,
	listUserDownstream,
	adoptDownstream,
	AdminError,
	type DojoClient
} from './contributions-data';

// A chainable supabase-js stub. `.is`/`.order` are read terminals (resolve the
// result); `.in` returns the builder so it can be a terminal too (awaited via
// `.then`, e.g. the adopt update). Captures table + filters + the update patch.
function makeDb(result: { data?: unknown; error: unknown }) {
	const captured: {
		table?: string;
		eqs: [string, unknown][];
		ins: [string, unknown][];
		update?: Record<string, unknown>;
	} = { eqs: [], ins: [] };
	const b: Record<string, unknown> = {};
	b.from = (t: string) => {
		captured.table = t;
		return b;
	};
	b.select = () => b;
	b.update = (u: Record<string, unknown>) => {
		captured.update = u;
		return b;
	};
	b.eq = (c: string, v: unknown) => {
		captured.eqs.push([c, v]);
		return b;
	};
	b.in = (c: string, v: unknown) => {
		captured.ins.push([c, v]);
		return b;
	};
	b.is = () => Promise.resolve(result);
	b.order = () => Promise.resolve(result);
	b.then = (resolve: (v: unknown) => unknown) => resolve(result);
	return { db: b as unknown as DojoClient, captured };
}

describe('userMembershipIds', () => {
	it('returns the caller active membership ids', async () => {
		const { db, captured } = makeDb({ data: [{ id: 'm1' }, { id: 'm2' }], error: null });
		expect(await userMembershipIds(db, 'u1')).toEqual(['m1', 'm2']);
		expect(captured.table).toBe('memberships');
		expect(captured.eqs).toContainEqual(['user_id', 'u1']);
	});
	it('fails closed (500) on error', async () => {
		const { db } = makeDb({ data: null, error: { message: 'boom' } });
		await expect(userMembershipIds(db, 'u1')).rejects.toBeInstanceOf(AdminError);
	});
});

describe('listUserContributions — mine (artifacts contributed_by the user)', () => {
	it('scopes by contributed_by and flattens the tenant name into dest', async () => {
		const { db, captured } = makeDb({
			data: [
				{ kind: 'pattern', title: 't', status: 'published', attribution: { mode: 'anonymous' }, scope: {}, created_at: 'x', tenant: { name: 'Acme' } }
			],
			error: null
		});
		const rows = await listUserContributions(db, 'u1');
		expect(captured.table).toBe('artifacts');
		expect(captured.eqs).toContainEqual(['contributed_by', 'u1']);
		expect(rows[0]).toMatchObject({ kind: 'pattern', status: 'published', dest: 'Acme' });
	});
	it('flattens a to-one tenant embedded as a 1-element array', async () => {
		const { db } = makeDb({
			data: [{ kind: 'guard', title: 't', status: 'submitted', attribution: null, scope: null, created_at: 'x', tenant: [{ name: 'Globex' }] }],
			error: null
		});
		expect((await listUserContributions(db, 'u1'))[0].dest).toBe('Globex');
	});
	it('honest-empty [] on null data; fails closed on error', async () => {
		expect(await listUserContributions(makeDb({ data: null, error: null }).db, 'u1')).toEqual([]);
		await expect(listUserContributions(makeDb({ data: null, error: { message: 'x' } }).db, 'u1')).rejects.toBeInstanceOf(AdminError);
	});
});

describe('listUserDownstream — approved for you (inbox ⋈ artifact)', () => {
	it('short-circuits to [] for no memberships (no rows can be theirs) — no query', async () => {
		const { db, captured } = makeDb({ data: null, error: { message: 'should not run' } });
		expect(await listUserDownstream(db, [])).toEqual([]);
		expect(captured.table).toBeUndefined();
	});
	it('filters by membership_id IN and flattens the artifact join', async () => {
		const { db, captured } = makeDb({
			data: [{ artifact_id: 'art-9', state: 'pinned', created_at: 'x', artifact: { kind: 'skill', title: 'S', scope: {}, tenant: { name: 'Acme' } } }],
			error: null
		});
		const rows = await listUserDownstream(db, ['m1', 'm2']);
		expect(captured.table).toBe('downstream_inbox');
		expect(captured.ins).toContainEqual(['membership_id', ['m1', 'm2']]);
		expect(rows[0]).toMatchObject({ id: 'art-9', state: 'pinned', kind: 'skill', title: 'S', from: 'Acme' });
	});
	it('fails closed on error', async () => {
		await expect(listUserDownstream(makeDb({ data: null, error: { message: 'x' } }).db, ['m1'])).rejects.toBeInstanceOf(AdminError);
	});
});

describe('adoptDownstream — Pin write (own rows only)', () => {
	it('flips the caller inbox rows for the artifact to pinned', async () => {
		const { db, captured } = makeDb({ error: null });
		await adoptDownstream(db, ['m1', 'm2'], 'art-1');
		expect(captured.update).toMatchObject({ state: 'pinned' });
		expect(captured.eqs).toContainEqual(['artifact_id', 'art-1']);
		expect(captured.ins).toContainEqual(['membership_id', ['m1', 'm2']]);
	});
	it('403 for no memberships (never a silent no-op success)', async () => {
		const err = await adoptDownstream(makeDb({ error: null }).db, [], 'a').catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect((err as AdminError).status).toBe(403);
	});
	it('fails closed (500) on a DB error', async () => {
		await expect(adoptDownstream(makeDb({ error: { message: 'x' } }).db, ['m1'], 'a')).rejects.toBeInstanceOf(AdminError);
	});
});
