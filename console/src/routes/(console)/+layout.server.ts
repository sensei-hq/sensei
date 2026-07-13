import type { LayoutServerLoad } from './$types';
import { resolveTenantKey, orgForTenant, TENANT_COOKIE, TENANT_PARAM } from '$lib/tenant';

// The tenant + auth surface for every guarded console screen. Mirrors the kavach
// demo's server-load pattern (sites/demo/src/routes/+layout.server.ts): read the
// session set by the kavach handle (hooks.server.ts) straight off `locals` and
// hand it to the pages via `data`.
//
//   • tenantKey  — the selected org's discovery path, read from the `dojo_tenant`
//                  cookie set on /orgs `enter()`. `?tenant=` is honoured as a dev
//                  override. Falls back to a safe default so the shell always
//                  renders (SSR/prerender/direct-link).
//   • org        — the static org record backing the tenant (chrome: name, kanji,
//                  role, members).
//   • accessToken — the session JWT (`locals.session.access_token`), passed to
//                  the triage API client as `Authorization: Bearer <token>`. Null
//                  when unauthenticated; the client then omits the header and the
//                  call surfaces the resulting 401 rather than crashing.
//
// A server load (not a universal `+layout.ts`) because only the server can read
// `locals` and the httpOnly-free-but-server-owned cookie.
export const load: LayoutServerLoad = ({ cookies, url, locals }) => {
	const tenantKey = resolveTenantKey(
		cookies.get(TENANT_COOKIE),
		url.searchParams.get(TENANT_PARAM)
	);
	return {
		tenantKey,
		org: orgForTenant(tenantKey),
		accessToken: locals.session?.access_token ?? null
	};
};
