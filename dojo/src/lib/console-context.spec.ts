import { describe, expect, it } from 'vitest';
import { deriveConsoleContext } from '$lib/console-context';
import { DEFAULT_TENANT_KEY } from '$lib/tenant';
import { orgs, type DojoOrg } from '$lib/dojo-data';

const member: DojoOrg = orgs[1]; // github/globex

describe('deriveConsoleContext — membership surfaced, no fabricated tenant (DJ1)', () => {
	it('a membership-less user has hasMembership false and NO tenant', () => {
		const ctx = deriveConsoleContext({ memberships: [], cookieTenant: null, paramTenant: null });
		expect(ctx.hasMembership).toBe(false);
		expect(ctx.tenantKey).toBeNull();
		expect(ctx.memberships).toEqual([]);
	});

	it("a member with no cookie/param defaults to their OWN first membership, not the fixture orgs[0]", () => {
		const ctx = deriveConsoleContext({
			memberships: [member],
			cookieTenant: null,
			paramTenant: null
		});
		expect(ctx.hasMembership).toBe(true);
		// The real membership (github/globex), NEVER the dojo-data fixture default —
		// scoping a real user to a fixture tenant makes every /v1/t/… call 404.
		expect(ctx.tenantKey).toBe(member.url);
		expect(ctx.tenantKey).not.toBe(DEFAULT_TENANT_KEY);
	});

	it("a member's persisted cookie is authoritative", () => {
		const ctx = deriveConsoleContext({
			memberships: [member],
			cookieTenant: 'github/globex',
			paramTenant: null
		});
		expect(ctx.hasMembership).toBe(true);
		expect(ctx.tenantKey).toBe('github/globex');
	});

	it('respects an explicit ?tenant= override even when membership-less (dev direct-link)', () => {
		const ctx = deriveConsoleContext({
			memberships: [],
			cookieTenant: null,
			paramTenant: 'other/initech'
		});
		// The user asked for a specific tenant — honoured — but still not a MEMBER.
		expect(ctx.hasMembership).toBe(false);
		expect(ctx.tenantKey).toBe('other/initech');
	});

	it('carries the signed-in user through for the personal home', () => {
		const ctx = deriveConsoleContext({
			memberships: [],
			cookieTenant: null,
			paramTenant: null,
			user: { name: 'Rin Saito', email: 'rin@x.dev' }
		});
		expect(ctx.user).toEqual({ name: 'Rin Saito', email: 'rin@x.dev' });
	});
	it('carries the user id through — the "you" chip resolves against membership user_ids', () => {
		const ctx = deriveConsoleContext({
			memberships: [],
			cookieTenant: null,
			paramTenant: null,
			user: { id: 'auth-uid-1', name: 'Rin Saito', email: 'rin@x.dev' }
		});
		expect(ctx.user?.id).toBe('auth-uid-1');
	});
});
