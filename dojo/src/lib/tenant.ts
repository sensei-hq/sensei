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
 * A safe fallback tenant key for a MEMBER who reached the console without an
 * explicit selection (static render / direct-link / test). Points at the demo
 * employer org. This default is applied ONLY when the caller has at least one
 * real membership — a membership-less user must never be handed a fabricated
 * tenant (see `resolveTenantKey` and DJ1); they land on the personal home.
 */
export const DEFAULT_TENANT_KEY = orgs[0]?.url ?? 'github/acme';

/**
 * The no-tenant sentinel: a signed-in user with zero memberships has no tenant.
 * Returned by `resolveTenantKey` so callers can branch (personal home) and skip
 * every `/v1/t/{tenant}/…` call rather than fetching against a fabricated org.
 */
export type TenantKey = string | null;

/** The discovery path a picked org routes to (its tenant key). */
export function tenantKeyOf(org: DojoOrg): string {
	return org.url;
}

/**
 * Resolve the tenant key from the persisted cookie (authoritative), falling back
 * to the legacy `?tenant=` param only when no cookie is set. Pure over its inputs
 * so it's testable and SSR-safe. Empty/whitespace values are ignored.
 *
 * When neither a cookie nor a param is present, the fallback depends on whether
 * the caller has a real membership:
 *   • `hasMembership === false` → `null` (the no-tenant sentinel). A signed-in
 *     user with zero memberships is NEVER defaulted onto `orgs[0]` (a fabricated
 *     tenant); they land on the personal home and make no `/v1/t/…` calls (DJ1).
 *   • otherwise (a member, or membership unknown — back-compat) → `DEFAULT_TENANT_KEY`.
 *
 * An explicit cookie/param override always wins, membership or not (a direct dev
 * link or a lingering selection reflects intent, not fabrication).
 */
export function resolveTenantKey(
	cookieValue?: string | null,
	paramValue?: string | null,
	hasMembership: boolean = true
): TenantKey {
	const cookie = cookieValue?.trim();
	if (cookie) return cookie;
	const param = paramValue?.trim();
	if (param) return param;
	return hasMembership ? DEFAULT_TENANT_KEY : null;
}

/**
 * Resolve the tenant key from a URL's search params, falling back to the default
 * when absent. Pure over its input so it's testable and SSR-safe. Retained for
 * the dev-override path (the cookie is resolved in the server load); this path
 * assumes a member context, so it always resolves a concrete key (never null).
 */
export function tenantKeyFromUrl(url: URL): string {
	return resolveTenantKey(null, url.searchParams.get(TENANT_PARAM), true) ?? DEFAULT_TENANT_KEY;
}

/** The org record backing a tenant key (for chrome: name, kanji, role, members). */
export function orgForTenant(tenantKey: string): DojoOrg | undefined {
	return orgs.find((o) => o.url === tenantKey);
}
