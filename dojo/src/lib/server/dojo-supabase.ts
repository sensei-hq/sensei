// Server-only Supabase access for the in-Worker dojo API (`/v1/t/…` routes that
// replace the standalone dojo-mind service). Uses the SERVICE ROLE key — the
// routes enforce tenant + role authorization in code (see `dojo-auth.ts`),
// faithfully mirroring dojo-mind's `resolve_tenant_access`. Never import this
// from client code.
//
// Prerequisites (one-time, Supabase dashboard):
//   - Settings → API → Exposed schemas: add `dojo` (and `sensei`), so PostgREST
//     can read `dojo.tenants` / `memberships` / `engagements` / ….
//   - Worker env: SUPABASE_SERVICE_ROLE_KEY (private) + PUBLIC_SUPABASE_URL.

import { createClient } from '@supabase/supabase-js';
import { env as pub } from '$env/dynamic/public';
import { env as priv } from '$env/dynamic/private';

// Built lazily so a build / prerender without the env doesn't throw at import
// time. The `db.schema` option makes supabase-js infer a `dojo`-schema client
// type, so we keep the inferred return type rather than annotate `SupabaseClient`
// (whose default `public` schema wouldn't match).
function buildClient(url: string, key: string) {
	return createClient(url, key, {
		auth: { persistSession: false, autoRefreshToken: false },
		db: { schema: 'dojo' }
	});
}

let client: ReturnType<typeof buildClient> | null = null;

/** Service-role client scoped to the `dojo` schema. Bypasses RLS — authorization
 *  is the caller's responsibility (`dojo-auth.ts`). */
export function dojoDb(): ReturnType<typeof buildClient> {
	if (client) return client;
	const url = pub.PUBLIC_SUPABASE_URL;
	// The server key: Supabase's new **secret** key (`sb_secret_…`) or the legacy
	// service_role key — both bypass RLS and drop into createClient the same way.
	// Accept either env name (SUPABASE_SECRET_KEY is the new convention).
	const key = priv.SUPABASE_SERVICE_ROLE_KEY || priv.SUPABASE_SECRET_KEY;
	if (!url || !key) {
		throw new Error(
			'dojo API: PUBLIC_SUPABASE_URL and SUPABASE_SERVICE_ROLE_KEY (or SUPABASE_SECRET_KEY) must be set'
		);
	}
	client = buildClient(url, key);
	return client;
}

// Access ladder — mirrors dojo-mind `DojoAccess` (auth.rs). Higher = more.
export const ACCESS = { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 } as const;
export type AccessLevel = (typeof ACCESS)[keyof typeof ACCESS];

/** Map a `dojo.memberships.role` string to an access level (mirrors
 *  `dojo_role_to_access`). Unknown / missing → member (0). */
export function roleToAccess(role: string | null | undefined): AccessLevel {
	switch (role) {
		case 'admin': return ACCESS.admin;
		case 'maintainer': return ACCESS.maintainer;
		case 'lead': return ACCESS.lead;
		case 'contributor': return ACCESS.contributor;
		default: return ACCESS.member;
	}
}

export function accessLabel(level: AccessLevel): string {
	return (Object.keys(ACCESS) as (keyof typeof ACCESS)[]).find((k) => ACCESS[k] === level) ?? 'member';
}
