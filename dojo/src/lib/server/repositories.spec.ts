// Repository registration and the sync plan (§V.4, in the user-scoped shape
// §VIII.1 corrected it to). Runs against `fakeDojoDb` so the (tenant_id,
// repo_key) unique is real and re-registering is observably a no-op.
import { describe, it, expect, beforeEach } from 'vitest';
import { fakeDojoDb, resetFakeIds, type FakeTable } from './fake-dojo-db';
import { registerRepositories, syncPlan } from './repositories';

const ALICE = 'p-alice';

/** Alice is in `organization/acme` (connected to github org `acme`) and in her
 *  personal dōjō. `organization/other` exists but she is not a member. */
function tables(): Record<string, FakeTable> {
	return {
		memberships: {
			rows: [
				{ id: 'm1', tenant_id: 't-acme', user_id: ALICE },
				{ id: 'm2', tenant_id: 't-personal', user_id: ALICE },
				{ id: 'm3', tenant_id: 't-other', user_id: 'p-bob' }
			]
		},
		tenants: {
			rows: [
				{ id: 't-acme', key: 'organization/acme', origin: 'organization', slug: 'acme' },
				{ id: 't-personal', key: 'personal/alice', origin: 'personal', slug: 'alice' },
				{ id: 't-other', key: 'organization/other', origin: 'organization', slug: 'other' }
			]
		},
		tenant_connections: {
			rows: [
				{ id: 'c1', tenant_id: 't-acme', provider: 'github', external_id: '11', external_slug: 'acme' },
				{ id: 'c2', tenant_id: 't-other', provider: 'github', external_id: '22', external_slug: 'secret-org' }
			]
		},
		repositories: {
			rows: [],
			uniques: [{ columns: ['tenant_id', 'repo_key'] }]
		}
	};
}

beforeEach(() => resetFakeIds());

describe('registerRepositories', () => {
	it('maps a repo to the tenant its forge org is connected to', async () => {
		const db = fakeDojoDb(tables());
		const out = await registerRepositories(db as never, ALICE, [
			{ repo_key: 'github.com/acme/api', remote_url: 'git@github.com:acme/api.git', name: 'api' }
		]);
		expect(out.unmapped).toEqual([]);
		expect(out.mapped[0]).toMatchObject({
			repo_key: 'github.com/acme/api',
			tenant: 'organization/acme'
		});
		expect(db.tables.repositories.rows[0]).toMatchObject({
			tenant_id: 't-acme',
			repo_key: 'github.com/acme/api',
			name: 'api'
		});
	});

	it('is idempotent — re-registering returns the same row, not a second one', async () => {
		const db = fakeDojoDb(tables());
		const first = await registerRepositories(db as never, ALICE, [
			{ repo_key: 'github.com/acme/api' }
		]);
		const second = await registerRepositories(db as never, ALICE, [
			{ repo_key: 'github.com/acme/api' }
		]);
		expect(db.tables.repositories.rows).toHaveLength(1);
		expect(second.mapped[0].repo_id).toBe(first.mapped[0].repo_id);
	});

	it('derives a name from the key when the daemon sent none', async () => {
		// `name` is NOT NULL. The last segment IS the repository's name, so this
		// is derived from the identity we were given, not invented.
		const db = fakeDojoDb(tables());
		await registerRepositories(db as never, ALICE, [{ repo_key: 'github.com/acme/deep-repo' }]);
		expect(db.tables.repositories.rows[0].name).toBe('deep-repo');
	});

	it('leaves an unrecognised host UNMAPPED, and writes no row', async () => {
		// §II.6: unmapped is NOT personal. Defaulting a self-hosted remote to the
		// caller's personal dōjō would move an employer's private repository into
		// a free personal tenant.
		const db = fakeDojoDb(tables());
		const out = await registerRepositories(db as never, ALICE, [
			{ repo_key: 'git.internal.acme.com/acme/api' }
		]);
		expect(out.mapped).toEqual([]);
		expect(out.unmapped).toEqual([{ repo_key: 'git.internal.acme.com/acme/api', reason: 'unknown_host' }]);
		expect(db.tables.repositories.rows).toHaveLength(0);
	});

	it('reports no_connection when the org has never been connected', async () => {
		const db = fakeDojoDb(tables());
		const out = await registerRepositories(db as never, ALICE, [
			{ repo_key: 'github.com/never-seen/api' }
		]);
		expect(out.unmapped).toEqual([{ repo_key: 'github.com/never-seen/api', reason: 'no_connection' }]);
	});

	it('refuses to register into a tenant the caller does not belong to', async () => {
		// The authorization boundary. `secret-org` IS connected — to a tenant
		// Alice is not in. Without this check anyone could push repositories into
		// any tenant that happened to have connected their org.
		const db = fakeDojoDb(tables());
		const out = await registerRepositories(db as never, ALICE, [
			{ repo_key: 'github.com/secret-org/private-api' }
		]);
		expect(out.unmapped).toEqual([
			{ repo_key: 'github.com/secret-org/private-api', reason: 'not_a_member' }
		]);
		expect(db.tables.repositories.rows).toHaveLength(0);
	});

	it('reports ambiguous rather than guessing when a slug resolves to two of my tenants', async () => {
		// Only possible when an org was renamed and the name re-registered
		// upstream. Picking one would attach real code to the wrong governance
		// boundary — a silent, invisible mis-routing.
		const t = tables();
		t.tenant_connections.rows.push({
			id: 'c3',
			tenant_id: 't-personal',
			provider: 'github',
			external_id: '33',
			external_slug: 'ACME' // case-insensitively the same slug
		});
		const db = fakeDojoDb(t);
		const out = await registerRepositories(db as never, ALICE, [{ repo_key: 'github.com/acme/api' }]);
		expect(out.unmapped).toEqual([{ repo_key: 'github.com/acme/api', reason: 'ambiguous' }]);
		expect(db.tables.repositories.rows).toHaveLength(0);
	});

	it('handles a mixed batch without letting one bad repo stop the rest', async () => {
		const db = fakeDojoDb(tables());
		const out = await registerRepositories(db as never, ALICE, [
			{ repo_key: 'github.com/acme/api' },
			{ repo_key: 'git.internal/x/y' },
			{ repo_key: 'github.com/acme/web' }
		]);
		expect(out.mapped.map((m) => m.repo_key)).toEqual([
			'github.com/acme/api',
			'github.com/acme/web'
		]);
		expect(out.unmapped).toHaveLength(1);
	});
});

describe('syncPlan', () => {
	it('allows every registered repo in phase 1, with denied present but empty', async () => {
		// `denied: []` rather than absent, so the daemon's handling of it is
		// exercised from day one and phase 2 changes no shape (§V.5).
		const db = fakeDojoDb(tables());
		await registerRepositories(db as never, ALICE, [
			{ repo_key: 'github.com/acme/api' },
			{ repo_key: 'github.com/acme/web' }
		]);
		const plan = await syncPlan(db as never, ALICE);
		expect(plan.allowed.map((a) => a.repo_key).sort()).toEqual([
			'github.com/acme/api',
			'github.com/acme/web'
		]);
		expect(plan.allowed[0].tenant).toBe('organization/acme');
		expect(plan.denied).toEqual([]);
	});

	it('never lists a repository from a tenant the caller is not in', async () => {
		// The plan is an ALLOW-LIST the daemon acts on directly, so a leak here
		// is not a display bug — it is the daemon syncing someone else's code.
		const t = tables();
        t.repositories.rows.push({
			id: 'r-other',
			tenant_id: 't-other',
			repo_key: 'github.com/secret-org/private-api',
			name: 'private-api'
		});
		const db = fakeDojoDb(t);
		const plan = await syncPlan(db as never, ALICE);
		expect(plan.allowed.map((a) => a.repo_key)).not.toContain('github.com/secret-org/private-api');
	});

	it('returns an empty plan for a user with no memberships', async () => {
		// Genuinely empty: no tenants, so nothing to sync. Offline degrades the
		// same way by construction — no plan, no sync.
		const db = fakeDojoDb(tables());
		expect(await syncPlan(db as never, 'p-nobody')).toEqual({ allowed: [], denied: [] });
	});
});
