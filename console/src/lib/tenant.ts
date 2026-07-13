// Selected-tenant plumbing for the console (R9).
//
// ⚠️ GAP FLAGGED FOR JERRY — R6 does NOT persist the org chosen in /orgs: its
// `enter()` handler just `goto('/console')` and drops the selection. There is no
// org→console tenant threading yet. Until R6/R11 wires a real per-org session
// (e.g. the selected tenant on `event.locals`, or a cookie), the console reads
// the tenant key from a `?tenant=<origin/org>` query param, which /orgs now
// appends from the picked org's discovery `url`. This is a PLACEHOLDER transport,
// not the final design — it is unauthenticated-safe (SSR/prerender/tests read a
// default) and easy to rip out when the session carries the tenant.
//
// The tenant KEY is the dojo discovery path `<origin>/<org>[/<dojo>]` (see
// dojo-mind api.rs `resolve_tenant`), which is exactly the `DojoOrg.url` the
// picker already shows for SaaS orgs (`github/globex`, `other/initech`).

import { orgs, type DojoOrg } from './dojo-data';

/** The query-param name that carries the selected tenant into the console. */
export const TENANT_PARAM = 'tenant';

/**
 * A safe fallback tenant key so the console renders without a selection (static
 * render / direct-link / test). Points at the demo employer org. Reaching the
 * console with no `?tenant=` means the org picker was bypassed — the screen shows
 * a "pick an organization" affordance rather than fabricating a live tenant.
 */
export const DEFAULT_TENANT_KEY = orgs[0]?.url ?? 'github/acme';

/** The discovery path a picked org routes to (its tenant key). */
export function tenantKeyOf(org: DojoOrg): string {
	return org.url;
}

/**
 * Resolve the tenant key from a URL's search params, falling back to the default
 * when absent. Pure over its input so it's testable and SSR-safe.
 */
export function tenantKeyFromUrl(url: URL): string {
	return url.searchParams.get(TENANT_PARAM) ?? DEFAULT_TENANT_KEY;
}

/** The org record backing a tenant key (for chrome: name, kanji, role, members). */
export function orgForTenant(tenantKey: string): DojoOrg | undefined {
	return orgs.find((o) => o.url === tenantKey);
}
