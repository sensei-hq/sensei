import { describe, expect, it, vi } from 'vitest';
import { orgs } from '$lib/dojo-data';
import { orgHref } from '$lib/dojo2-nav';
import {
	DEFAULT_TENANT_KEY,
	TENANT_COOKIE,
	TENANT_PARAM,
	enterOrg,
	orgForTenant,
	resolveTenantKey,
	tenantCookieString,
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

describe('tenantCookieString / enterOrg (the one shared tenant-switch path)', () => {
	it('encodes the org tenant key into the dojo_tenant session cookie', () => {
		const cookie = tenantCookieString('github/globex');
		expect(cookie).toBe(`${TENANT_COOKIE}=github%2Fglobex; path=/; SameSite=Lax`);
	});

	it('enterOrg writes the tenant cookie for the picked org then navigates to the console', () => {
		const setCookie = vi.fn();
		const navigate = vi.fn();
		const org = orgs[1]; // Globex → github/globex
		const landed = enterOrg(org, {
			setCookie,
			navigate,
			consoleHref: '/(console)/console'
		});
		expect(setCookie).toHaveBeenCalledTimes(1);
		expect(setCookie).toHaveBeenCalledWith(
			`${TENANT_COOKIE}=${encodeURIComponent(org.url)}; path=/; SameSite=Lax`
		);
		expect(navigate).toHaveBeenCalledWith('/(console)/console');
		expect(landed).toBe('/(console)/console');
	});
});

describe('/orgs Enter → dojo2 org context (landing cutover)', () => {
	// The org picker's `enter(org)` composes `enterOrg` with `orgHref(org.id)` — the
	// landing cutover routes a picked org to the dojo2 ORG route (/org/{slug}), NOT
	// the old /console. The slug is the org `id`, the SAME value `orgBySlug` and the
	// /org/[slug] load resolve against, so entering lands on a valid org route.
	it('navigates to /org/{id} (not /console) and still sets the tenant cookie', () => {
		const setCookie = vi.fn();
		const navigate = vi.fn();
		const org = orgs[1]; // Globex → id "globex", url "github/globex"

		enterOrg(org, { setCookie, navigate, consoleHref: orgHref(org.id) });

		// Lands on the dojo2 org context, never the retired /console path.
		expect(navigate).toHaveBeenCalledWith(`/org/${org.id}`);
		expect(navigate).toHaveBeenCalledWith(expect.stringMatching(/^\/org\//));
		const dest = navigate.mock.calls[0][0] as string;
		expect(dest.startsWith('/console')).toBe(false);

		// The shared tenant-switch still persists the dojo_tenant cookie (tenant key
		// = org.url), so the resolved /org/[slug] context has its selection.
		expect(setCookie).toHaveBeenCalledWith(
			`${TENANT_COOKIE}=${encodeURIComponent(org.url)}; path=/; SameSite=Lax`
		);
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
