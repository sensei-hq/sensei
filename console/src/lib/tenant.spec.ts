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

describe('resolveTenantKey (cookie is authoritative)', () => {
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

	it('falls back to the default when neither cookie nor param is present', () => {
		expect(resolveTenantKey(null, null)).toBe(DEFAULT_TENANT_KEY);
		expect(resolveTenantKey(undefined, undefined)).toBe(DEFAULT_TENANT_KEY);
		expect(resolveTenantKey('', '')).toBe(DEFAULT_TENANT_KEY);
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
