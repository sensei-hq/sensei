import { describe, expect, it } from 'vitest';
import { orgs } from '$lib/dojo-data';
import {
	DEFAULT_TENANT_KEY,
	TENANT_PARAM,
	orgForTenant,
	resolveTenantKey,
	tenantKeyFromUrl,
	tenantKeyOf
} from '$lib/tenant';

describe('tenant key plumbing', () => {
	it('a picked org routes to its discovery url as the tenant key', () => {
		const org = orgs[1];
		expect(tenantKeyOf(org)).toBe(org.url);
	});

	it('resolves the org record backing a tenant key', () => {
		const key = orgs[1].url;
		expect(orgForTenant(key)?.id).toBe(orgs[1].id);
		expect(orgForTenant('nope/unknown')).toBeUndefined();
	});
});

describe('resolveTenantKey (cookie is authoritative, for members)', () => {
	// A member (hasMembership omitted or true) keeps the original behaviour.
	it('reads the tenant from the cookie', () => {
		expect(resolveTenantKey('github/globex', null)).toBe('github/globex');
	});

	it('picks the cookie over the legacy ?tenant= param', () => {
		expect(resolveTenantKey('github/globex', 'other/initech')).toBe('github/globex');
	});

	it('falls back to the legacy param only when no cookie is set', () => {
		expect(resolveTenantKey(null, 'other/initech')).toBe('other/initech');
		expect(resolveTenantKey('', 'other/initech')).toBe('other/initech');
		expect(resolveTenantKey('   ', 'other/initech')).toBe('other/initech');
	});

	it('falls back to the default only when the caller HAS a membership', () => {
		expect(resolveTenantKey(null, null, true)).toBe(DEFAULT_TENANT_KEY);
		expect(resolveTenantKey(undefined, undefined, true)).toBe(DEFAULT_TENANT_KEY);
		expect(resolveTenantKey('', '', true)).toBe(DEFAULT_TENANT_KEY);
	});
});

describe('resolveTenantKey (membership-less user is never handed a fabricated tenant)', () => {
	// DJ1: a signed-in user with zero memberships must NOT be defaulted onto
	// orgs[0] (Acme). With no cookie/param and hasMembership === false, there is
	// no tenant — the console shows the personal home, never a live tenant call.
	it('returns null when there is no cookie/param and no membership', () => {
		expect(resolveTenantKey(null, null, false)).toBeNull();
		expect(resolveTenantKey(undefined, undefined, false)).toBeNull();
		expect(resolveTenantKey('', '', false)).toBeNull();
		expect(resolveTenantKey('   ', '   ', false)).toBeNull();
	});

	it('still honours an explicit cookie/param override even without a membership', () => {
		// A direct dev link (?tenant=) or a lingering cookie is respected — the
		// no-tenant sentinel only fills the empty case, it does not override intent.
		expect(resolveTenantKey('github/globex', null, false)).toBe('github/globex');
		expect(resolveTenantKey(null, 'other/initech', false)).toBe('other/initech');
	});

	it('defaults to orgs[0] when hasMembership is left unspecified (back-compat)', () => {
		expect(resolveTenantKey(null, null)).toBe(DEFAULT_TENANT_KEY);
	});
});

describe('tenantKeyFromUrl (dev-override fallback)', () => {
	it('reads the tenant from the ?tenant= query param', () => {
		const url = new URL(`https://console.test/console?${TENANT_PARAM}=github%2Fglobex`);
		expect(tenantKeyFromUrl(url)).toBe('github/globex');
	});

	it('falls back to the default tenant when the param is absent', () => {
		const url = new URL('https://console.test/console');
		expect(tenantKeyFromUrl(url)).toBe(DEFAULT_TENANT_KEY);
	});
});
