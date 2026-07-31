// The shared server-load body for every guarded shell (the shipped `(console)`
// group AND the new `(app)` group). Extracted so the two shells resolve the
// exact same tenant + membership + auth surface from ONE place — they can never
// drift (DRY). The pure decision lives in `deriveConsoleContext`; this helper
// does only the I/O (Supabase memberships, cookie read, chrome-org lookup).
//
//   • hasMembership — the caller's REAL memberships (`listUserOrgs`), NOT a
//                  tenant fallback. False for a signed-in user with zero
//                  memberships (DJ1). The shells branch on this (personal
//                  landing when false).
//   • tenantKey  — the selected org's discovery path (`dojo_tenant` cookie;
//                  `?tenant=` honoured as a dev override). NULL for a
//                  membership-less user with no explicit override.
//   • org        — the real tenant the caller is entering, when a member.
//   • memberships — the caller's Dōjōs (for the org switcher; empty when solo).
//   • user       — the signed-in identity (for the personal greeting).
//   • accessToken — the session JWT, passed to the API clients as a bearer.

import type { Cookies } from '@sveltejs/kit';
import { TENANT_COOKIE, TENANT_PARAM, type TenantKey } from '$lib/tenant';
import { deriveConsoleContext } from '$lib/console-context';
import { getUserOrg, listUserOrgs, sessionUser, userProfile } from '$lib/server/dojo-orgs';
import type { DojoOrg } from '$lib/dojo-data';
import type { PersonalUser } from '$lib/personal-home-view';

/** The shape every guarded shell hands to its pages via `data`. */
export interface ConsoleLoadData {
	tenantKey: TenantKey;
	hasMembership: boolean;
	memberships: DojoOrg[];
	user?: PersonalUser;
	org?: DojoOrg;
	accessToken: string | null;
}

/** Run the shared console load against a request's cookies/url/locals. */
export async function loadConsoleContext({
	cookies,
	url,
	locals
}: {
	cookies: Cookies;
	url: URL;
	locals: App.Locals;
}): Promise<ConsoleLoadData> {
	const su = sessionUser(locals);
	const memberships = su?.id ? await listUserOrgs(su.id) : [];
	const profile = userProfile(su);
	const ctx = deriveConsoleContext({
		memberships,
		cookieTenant: cookies.get(TENANT_COOKIE),
		paramTenant: url.searchParams.get(TENANT_PARAM),
		user: { id: su?.id ?? null, name: profile.name, email: profile.handle }
	});
	// Resolve the chrome org only when there's a tenant to resolve (a member with
	// a selection); a membership-less user has no tenant, so no lookup is made.
	const org = su?.id && ctx.tenantKey ? await getUserOrg(su.id, ctx.tenantKey) : undefined;
	return {
		tenantKey: ctx.tenantKey,
		hasMembership: ctx.hasMembership,
		memberships: ctx.memberships,
		user: ctx.user,
		org,
		accessToken: locals.session?.access_token ?? null
	};
}
