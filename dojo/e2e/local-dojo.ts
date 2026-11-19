// The local dōjō the browser specs run against, and the fixtures they seed in it.
//
// LOCAL ONLY. The keys come from `dojo/.dev.vars` (gitignored), and `assertLocal`
// refuses any Supabase URL that is not on this machine, so a spec can never seed
// or sign in against a hosted project.
import { readFileSync } from 'node:fs';
import { createClient, type SupabaseClient } from '@supabase/supabase-js';

export const DOJO_URL = 'http://127.0.0.1:5175';

const NO_SESSION = { auth: { persistSession: false, autoRefreshToken: false } };

function devVars(): Record<string, string> {
	const path = new URL('../.dev.vars', import.meta.url);
	let text: string;
	try {
		text = readFileSync(path, 'utf8');
	} catch (e) {
		throw new Error(`dojo/.dev.vars is missing; the dōjō e2e needs the local Supabase keys (${String(e)})`);
	}
	const out: Record<string, string> = {};
	for (const line of text.split('\n')) {
		const m = line.match(/^\s*([A-Z0-9_]+)\s*=\s*(.*?)\s*$/);
		if (m) out[m[1]] = m[2].replace(/^["']|["']$/g, '');
	}
	return out;
}

function required(vars: Record<string, string>, name: string): string {
	const v = vars[name];
	if (!v) throw new Error(`dojo/.dev.vars has no ${name}`);
	return v;
}

function assertLocal(url: string): string {
	const host = new URL(url).hostname;
	if (host !== '127.0.0.1' && host !== 'localhost') {
		throw new Error(`the dōjō e2e runs against the local Supabase only, not ${host}`);
	}
	return url;
}

export type LocalDojo = {
	supabaseUrl: string;
	anonKey: string;
	/** Service role on the `dojo` schema: seeds and inspects, bypassing RLS. */
	// eslint-disable-next-line @typescript-eslint/no-explicit-any -- untyped schema
	db: SupabaseClient<any, 'dojo', 'dojo'>;
	/** Service role on `auth`: creates the test logins and mints their links. */
	admin: SupabaseClient;
};

export function localDojo(): LocalDojo {
	const vars = devVars();
	const supabaseUrl = assertLocal(required(vars, 'PUBLIC_SUPABASE_URL'));
	const anonKey = required(vars, 'PUBLIC_SUPABASE_ANON_KEY');
	const serviceKey = required(vars, 'SUPABASE_SERVICE_ROLE_KEY');
	return {
		supabaseUrl,
		anonKey,
		db: createClient(supabaseUrl, serviceKey, { ...NO_SESSION, db: { schema: 'dojo' } }),
		admin: createClient(supabaseUrl, serviceKey, NO_SESSION)
	};
}

type Outcome = { data: unknown; error: { message: string } | null };
/** The `data` of a Supabase result's success branch. */
type Success<R extends Outcome> = [Extract<R, { error: null }>] extends [never]
	? R['data']
	: Extract<R, { error: null }>['data'];

/** Throw on a Supabase error; a failed seed must stop the spec, not skew it. */
export function ok<R extends Outcome>(res: R, what: string): Success<R> {
	if (res.error) throw new Error(`${what}: ${res.error.message}`);
	return res.data as Success<R>;
}

export type TestUser = { email: string; authUserId: string; principalId: string };

/** A confirmed local login with its dōjō principal. */
export async function createUser(dojo: LocalDojo, email: string, displayName: string): Promise<TestUser> {
	const created = ok(await dojo.admin.auth.admin.createUser({ email, email_confirm: true }), `create login ${email}`);
	const authUserId = created.user!.id;
	const principal = ok(
		await dojo.db
			.from('principals')
			.insert({ auth_user_id: authUserId, display_name: displayName })
			.select('id')
			.single(),
		`create principal for ${email}`
	) as { id: string };
	return { email, authUserId, principalId: principal.id };
}

/** A magic link that signs `email` in and lands on `path` of the local dōjō. */
export async function magicLink(dojo: LocalDojo, email: string, path: string): Promise<string> {
	const link = ok(
		await dojo.admin.auth.admin.generateLink({
			type: 'magiclink',
			email,
			options: { redirectTo: `${DOJO_URL}${path}` }
		}),
		`magic link for ${email}`
	);
	return link.properties.action_link;
}

/** A session access token for `email`: what the daemon presents to the dōjō
 *  (`Authorization: Bearer`, see src/lib/server/dojo-auth.ts). */
export async function accessToken(dojo: LocalDojo, email: string): Promise<string> {
	const link = ok(await dojo.admin.auth.admin.generateLink({ type: 'magiclink', email }), `token link for ${email}`);
	const anon = createClient(dojo.supabaseUrl, dojo.anonKey, NO_SESSION);
	const verified = ok(
		await anon.auth.verifyOtp({ token_hash: link.properties.hashed_token, type: 'magiclink' }),
		`verify token link for ${email}`
	);
	const token = verified.session?.access_token;
	if (!token) throw new Error(`no session for ${email}`);
	return token;
}

/** Remove every login, principal and tenant whose name starts with `prefix`:
 *  this run's, and the leftovers of an earlier, interrupted run. */
export async function removeFixtures(dojo: LocalDojo, prefix: string): Promise<void> {
	const tenants = ok(await dojo.db.from('tenants').select('id').like('slug', `${prefix}%`), 'list fixture tenants') as {
		id: string;
	}[];
	const tenantIds = tenants.map((t) => t.id);
	if (tenantIds.length) {
		ok(await dojo.db.from('repositories_in_workspaces').delete().in('tenant_id', tenantIds), 'delete fixture links');
		ok(await dojo.db.from('workspaces').delete().in('tenant_id', tenantIds), 'delete fixture workspaces');
		const repos = ok(
			await dojo.db.from('repositories').select('id').in('tenant_id', tenantIds),
			'list fixture repositories'
		) as { id: string }[];
		if (repos.length) {
			const ids = repos.map((r) => r.id);
			ok(await dojo.db.from('repository_roles').delete().in('repository_id', ids), 'delete fixture repository roles');
		}
		ok(await dojo.db.from('repositories').delete().in('tenant_id', tenantIds), 'delete fixture repositories');
		ok(await dojo.db.from('tenant_connections').delete().in('tenant_id', tenantIds), 'delete fixture connections');
		ok(await dojo.db.from('memberships').delete().in('tenant_id', tenantIds), 'delete fixture memberships');
		ok(await dojo.db.from('tenants').delete().in('id', tenantIds), 'delete fixture tenants');
	}
	const users = ok(await dojo.admin.auth.admin.listUsers({ perPage: 1000 }), 'list logins');
	for (const u of users.users.filter((x) => x.email?.startsWith(prefix))) {
		const principals = ok(
			await dojo.db.from('principals').select('id').eq('auth_user_id', u.id),
			`list principal ${u.email}`
		) as { id: string }[];
		const ids = principals.map((p) => p.id);
		if (ids.length) {
			// Signing in also provisions memberships elsewhere (the personal dōjō).
			ok(await dojo.db.from('memberships').delete().in('user_id', ids), `delete memberships of ${u.email}`);
		}
		ok(await dojo.db.from('principals').delete().eq('auth_user_id', u.id), `delete principal ${u.email}`);
		ok(await dojo.admin.auth.admin.deleteUser(u.id), `delete login ${u.email}`);
	}
}
