// The pure decision behind the console layout load (DJ1): given the caller's
// real memberships + the selected-tenant cookie/param + their identity, derive
// the console context the shell + pages branch on. Kept pure (no Supabase, no
// `locals`) so it unit-tests without a backend; the server load does the I/O
// (Supabase memberships, cookie read) and delegates the decision here.
//
// The membership fact — NOT the tenant fallback — is the source of truth for
// `hasMembership`. A membership-less user is never handed a fabricated tenant:
// `resolveTenantKey(..., hasMembership=false)` returns `null` unless an explicit
// cookie/param override is present.
import { resolveTenantKey, type TenantKey } from './tenant';
import type { DojoOrg } from './dojo-data';
import type { PersonalUser } from './personal-home-view';

export interface ConsoleContextInput {
	/** The caller's real memberships (from `listUserOrgs`). Empty ⇒ solo. */
	memberships: DojoOrg[];
	/** The persisted selected-tenant cookie value (authoritative), or null. */
	cookieTenant?: string | null;
	/** The legacy `?tenant=` override, consulted only when no cookie is set. */
	paramTenant?: string | null;
	/** The signed-in user (for the personal-home greeting), if any. */
	user?: PersonalUser;
}

export interface ConsoleContext {
	/** True iff the caller has at least one real membership. */
	hasMembership: boolean;
	/** The tenant key to scope `/v1/t/…` calls to — `null` when membership-less
	 *  and no explicit override (the personal-home / no-fetch signal). */
	tenantKey: TenantKey;
	/** The caller's memberships, passed through for the org switcher. */
	memberships: DojoOrg[];
	/** The signed-in user, passed through for the personal home. */
	user?: PersonalUser;
}

/** Derive the console context from real memberships + the selected-tenant
 *  cookie/param. Pure and SSR-safe. */
export function deriveConsoleContext(input: ConsoleContextInput): ConsoleContext {
	const hasMembership = input.memberships.length > 0;
	// Default a member with no explicit cookie/param to their FIRST REAL membership
	// (already fetched via listUserOrgs), not the dojo-data fixture — otherwise a
	// real user (e.g. personal/jerry) gets scoped to a fixture tenant that 404s.
	const memberDefault = input.memberships[0]?.url ?? null;
	const tenantKey = resolveTenantKey(input.cookieTenant, input.paramTenant, hasMembership, memberDefault);
	return { hasMembership, tenantKey, memberships: input.memberships, user: input.user };
}
