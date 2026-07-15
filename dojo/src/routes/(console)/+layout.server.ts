import type { LayoutServerLoad } from './$types';
import { resolveTenantKey, TENANT_COOKIE, TENANT_PARAM } from '$lib/tenant';
import { getUserOrg, sessionUser } from '$lib/server/dojo-orgs';

// The tenant + auth surface for every guarded console screen. Mirrors the kavach
// demo's server-load pattern (sites/demo/src/routes/+layout.server.ts): read the
// session set by the kavach handle (hooks.server.ts) straight off `locals` and
// hand it to the pages via `data`.
//
//   • tenantKey  — the selected org's discovery path, read from the `dojo_tenant`
//                  cookie set on /orgs `enter()`. `?tenant=` is honoured as a dev
//                  override. Falls back to a safe default so the shell always
//                  renders (SSR/prerender/direct-link).
//   • org        — the real tenant the caller is entering, resolved from their
//                  active `dojo.membership` (chrome: name, kanji, role, members).
//                  Undefined if they have no membership in the cookie's tenant.
//   • accessToken — the session JWT (`locals.session.access_token`), passed to
//                  the triage API client as `Authorization: Bearer <token>`. Null
//                  when unauthenticated; the client then omits the header and the
//                  call surfaces the resulting 401 rather than crashing.
//
// A server load (not a universal `+layout.ts`) because only the server can read
// `locals` and the httpOnly-free-but-server-owned cookie.
export const load: LayoutServerLoad = async ({ cookies, url, locals }) => {
	const tenantKey = resolveTenantKey(
		cookies.get(TENANT_COOKIE),
		url.searchParams.get(TENANT_PARAM)
	);
	const su = sessionUser(locals);
	const org = su?.id ? await getUserOrg(su.id, tenantKey) : undefined;
	return {
		tenantKey,
		org,
		accessToken: locals.session?.access_token ?? null
	};
};
