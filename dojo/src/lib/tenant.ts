// Selected-tenant plumbing for the console (R9 + R-wiring).
//
// The org chosen in /orgs is persisted as the SESSION TENANT via a `dojo_tenant`
// cookie (set on `enter()`; read server-side in `(console)/+layout.server.ts`).
// That cookie is the real transport. The tenant KEY is the dojo discovery path
// `<origin>/<org>[/<dojo>]` (see dojo-mind api.rs `resolve_tenant`), which is
// exactly the `DojoOrg.url` the picker shows for SaaS orgs (`github/globex`,
// `other/initech`).
//
// The cookie is authoritative. The `?tenant=` query param is retained ONLY as a
// legacy/dev fallback (direct-link / manual testing) — consulted solely when no
// tenant cookie is set yet. Everything degrades to a safe default so the console
// renders under SSR/prerender and in unit tests without a selection or backend.

import { orgs, type DojoOrg } from './dojo-data';

/**
 * The cookie that carries the selected org's tenant key across console
 * navigation. Not httpOnly (client actions may read it), path=/, SameSite=Lax.
 */
export const TENANT_COOKIE = 'dojo_tenant';

/**
 * The query-param name kept only as a legacy/dev fallback for the selected
 * tenant. The cookie is authoritative — the param is consulted solely when no
 * tenant cookie has been set yet (e.g. a direct link before the org picker).
 */
export const TENANT_PARAM = 'tenant';

/**
 * A safe fallback tenant key so the console renders without a selection (static
 * render / direct-link / test). Points at the demo employer org. Reaching the
 * console with no selected tenant means the org picker was bypassed — the screen
 * shows a "pick an organization" affordance rather than fabricating a live tenant.
 */
export const DEFAULT_TENANT_KEY = orgs[0]?.url ?? 'github/acme';

/** The discovery path a picked org routes to (its tenant key). */
export function tenantKeyOf(org: DojoOrg): string {
	return org.url;
}

/**
 * Resolve the tenant key from the persisted cookie (authoritative), falling back
 * to the legacy `?tenant=` param only when no cookie is set, then to the default
 * when neither is present. Pure over its inputs so it's testable and SSR-safe.
 * Empty/whitespace values are ignored.
 */
export function resolveTenantKey(cookieValue?: string | null, paramValue?: string | null): string {
	const cookie = cookieValue?.trim();
	if (cookie) return cookie;
	const param = paramValue?.trim();
	if (param) return param;
	return DEFAULT_TENANT_KEY;
}

/**
 * Resolve the tenant key from a URL's search params, falling back to the default
 * when absent. Pure over its input so it's testable and SSR-safe. Retained for
 * the dev-override path (the cookie is resolved in the server load).
 */
export function tenantKeyFromUrl(url: URL): string {
	return resolveTenantKey(null, url.searchParams.get(TENANT_PARAM));
}

/** The org record backing a tenant key (for chrome: name, kanji, role, members). */
export function orgForTenant(tenantKey: string): DojoOrg | undefined {
	return orgs.find((o) => o.url === tenantKey);
}
