import type { LayoutServerLoad } from './$types';
import { TENANT_COOKIE, TENANT_PARAM } from '$lib/tenant';
import { deriveConsoleContext } from '$lib/console-context';
import { getUserOrg, listUserOrgs, sessionUser, userProfile } from '$lib/server/dojo-orgs';

// The tenant + membership + auth surface for every guarded console screen.
// Mirrors the kavach demo's server-load pattern (sites/demo/src/routes/
// +layout.server.ts): read the session set by the kavach handle (hooks.server.ts)
// off `locals` and hand it to the pages via `data`.
//
//   • hasMembership — derived from the caller's REAL memberships
//                  (`listUserOrgs`), NOT the tenant fallback. False for a
//                  signed-in user with zero memberships (DJ1). The console index
//                  branches on this: personal home when false, Overview when true.
//   • tenantKey  — the selected org's discovery path, from the `dojo_tenant`
//                  cookie (`?tenant=` honoured as a dev override). NULL for a
//                  membership-less user with no explicit override — they are NEVER
//                  handed a fabricated tenant, and org-scoped loads skip the fetch.
//   • org        — the real tenant the caller is entering, resolved from their
//                  active `dojo.membership` (chrome: name, kanji, role, members).
//                  Undefined when membership-less or not a member of the tenant.
//   • memberships — the caller's Dōjōs (for the switcher; empty when solo).
//   • user       — the signed-in identity (for the personal-home greeting).
//   • accessToken — the session JWT, passed to the API clients as a bearer.
//
// A server load (not a universal `+layout.ts`) because only the server can read
// `locals` and the httpOnly-free-but-server-owned cookie.
export const load: LayoutServerLoad = async ({ cookies, url, locals }) => {
	const su = sessionUser(locals);
	const memberships = su?.id ? await listUserOrgs(su.id) : [];
	const profile = userProfile(su);
	const ctx = deriveConsoleContext({
		memberships,
		cookieTenant: cookies.get(TENANT_COOKIE),
		paramTenant: url.searchParams.get(TENANT_PARAM),
		user: { name: profile.name, email: profile.handle }
	});
	// Resolve the chrome org only when there's a tenant to resolve (a member with
	// a selection); a membership-less user has no tenant, so no lookup is made.
	const org =
		su?.id && ctx.tenantKey ? await getUserOrg(su.id, ctx.tenantKey) : undefined;
	return {
		tenantKey: ctx.tenantKey,
		hasMembership: ctx.hasMembership,
		memberships: ctx.memberships,
		user: ctx.user,
		org,
		accessToken: locals.session?.access_token ?? null
	};
};
