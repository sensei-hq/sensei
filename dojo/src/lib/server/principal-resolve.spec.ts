// The login → principal translation (spec dojo-auth-provisioning §VIII.2).
//
// `dojo.principals` is the stable identity every dōjō foreign key points at;
// `principals.auth_user_id` is a re-pointable pointer at the Supabase login. So
// every `user_id` column downstream holds a PRINCIPAL id, and something has to
// do the translation exactly once. This is that something.
//
// The fail-closed contract matters more than usual here: a fabricated or
// defaulted principal id would not error — it would silently attribute one
// human's work to another, or hand them another tenant's membership.
import { describe, it, expect } from 'vitest';

const { resolvePrincipalId } = await import('./principal-resolve');
const { AdminError } = await import('./admin-data');

type Terminal = { data: unknown; error: unknown };

/**
 * Chainable supabase-js stub. Each terminal (`maybeSingle` / `single`) shifts the
 * next queued result, so a test declares the sequence of round-trips it expects.
 * `inserts` records every insert payload for assertion.
 */
function makeDb(...results: Terminal[]) {
	const queue = [...results];
	const inserts: unknown[] = [];
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = () => b;
	b.insert = (payload: unknown) => {
		inserts.push(payload);
		return b;
	};
	const next = () => Promise.resolve(queue.shift() ?? { data: null, error: null });
	b.maybeSingle = next;
	b.single = next;
	b.inserts = inserts;
	b.remaining = () => queue.length;
	return b as Record<string, unknown> & { inserts: unknown[]; remaining: () => number };
}

const AUTH_USER = '11111111-1111-1111-1111-111111111111';
const PRINCIPAL = 'aaaaaaaa-1111-1111-1111-111111111111';

describe('resolvePrincipalId', () => {
	it('returns the existing principal for a known login, without inserting', async () => {
		// The hot path: every authenticated request after the first. Asserts the
		// REAL resolved id — not merely "a string" — so a resolver that returned
		// the login id unchanged (the bug this exists to prevent) fails here.
		const db = makeDb({ data: { id: PRINCIPAL }, error: null });
		const id = await resolvePrincipalId(db as never, AUTH_USER);
		expect(id).toBe(PRINCIPAL);
		expect(id).not.toBe(AUTH_USER);
		expect(db.inserts).toHaveLength(0);
	});

	it('creates the principal on a first sign-in and returns the new id', async () => {
		// A brand-new human has a login but no principal yet. Provisioning itself
		// authenticates through this resolver, so refusing here would mean a user
		// could never bootstrap.
		const db = makeDb(
			{ data: null, error: null }, // lookup miss
			{ data: { id: PRINCIPAL }, error: null } // insert … returning id
		);
		const id = await resolvePrincipalId(db as never, AUTH_USER, 'Alice');
		expect(id).toBe(PRINCIPAL);
		expect(db.inserts).toEqual([{ auth_user_id: AUTH_USER, display_name: 'Alice' }]);
	});

	it('re-reads instead of failing when a concurrent sign-in inserted first', async () => {
		// Part I Scenario 22 — two tabs signing in at once. `principals.auth_user_id`
		// is UNIQUE, so the loser of the race gets 23505; that is the constraint
		// doing its job, not an error to surface. Both tabs must converge on the
		// SAME principal.
		const db = makeDb(
			{ data: null, error: null }, // lookup miss
			{ data: null, error: { code: '23505', message: 'duplicate key' } }, // lost the race
			{ data: { id: PRINCIPAL }, error: null } // re-read: the winner's row
		);
		const id = await resolvePrincipalId(db as never, AUTH_USER);
		expect(id).toBe(PRINCIPAL);
	});

	it('throws 500 on a lookup error rather than minting an id', async () => {
		// Fail closed. A DB hiccup must not become "no principal, make one" — that
		// would fork a second principal for a human who already has one, orphaning
		// every membership the first one holds.
		const db = makeDb({ data: null, error: { message: 'db down' } });
		const err = await resolvePrincipalId(db as never, AUTH_USER).catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect(err.status).toBe(500);
		expect(db.inserts).toHaveLength(0);
	});

	it('throws 500 when the insert fails for a reason that is not the race', async () => {
		const db = makeDb(
			{ data: null, error: null },
			{ data: null, error: { code: '42501', message: 'permission denied' } }
		);
		const err = await resolvePrincipalId(db as never, AUTH_USER).catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect(err.status).toBe(500);
	});

	it('throws 500 when the post-race re-read still finds nothing', async () => {
		// Should be unreachable — 23505 means a row exists. If it is reachable
		// anyway, the honest answer is an error, never a synthesised id.
		const db = makeDb(
			{ data: null, error: null },
			{ data: null, error: { code: '23505', message: 'duplicate key' } },
			{ data: null, error: null }
		);
		const err = await resolvePrincipalId(db as never, AUTH_USER).catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect(err.status).toBe(500);
	});

	it('rejects an empty login id before touching the database', async () => {
		const db = makeDb();
		const err = await resolvePrincipalId(db as never, '').catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect(db.inserts).toHaveLength(0);
	});
});
