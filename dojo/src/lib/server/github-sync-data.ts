// GitHub-org auto-join (F3c). A user signed in via GitHub OAuth carries a GitHub
// access token in their Supabase session (`session.provider_token`). We use THAT
// token — the user's own, proving their own memberships — to read their GitHub
// orgs and auto-provision a membership in any matching `github/{org}` dōjō. Fail
// closed: only orgs the GitHub API confirms are provisioned (a leaked/absent token
// grants nothing), and we never provision an org the user isn't in. No stored App
// secret; the URL is the fixed api.github.com (no SSRF surface).
import { AdminError, addMember, type DojoClient } from './admin-data';

export { AdminError };
export type { DojoClient };

/** The user's GitHub org logins, read with THEIR OAuth token. Fails closed: a
 *  non-2xx GitHub response throws (the caller treats it as "no sync", never a
 *  fabricated org list). `fetchImpl` is injected for tests. */
export async function fetchGithubOrgLogins(
	token: string,
	fetchImpl: typeof fetch = fetch
): Promise<string[]> {
	const res = await fetchImpl('https://api.github.com/user/orgs?per_page=100', {
		headers: {
			Authorization: `Bearer ${token}`,
			Accept: 'application/vnd.github+json',
			'User-Agent': 'sensei-dojo'
		}
	});
	if (!res.ok) throw new AdminError(502, `GitHub org lookup failed (${res.status})`);
	const body = (await res.json()) as unknown;
	if (!Array.isArray(body)) return [];
	return body
		.map((o) => (o as { login?: unknown }).login)
		.filter((l): l is string => typeof l === 'string' && l.length > 0);
}

/**
 * Auto-provision the caller into the `github/{org}` dōjōs they belong to. Only
 * tenants that already exist AND that the caller isn't already a member of are
 * joined (fail-closed on both sides: never invents a tenant, never re-joins). The
 * membership is `github_oauth` / `employer` at `contributor`. Returns the tenant
 * keys joined this pass. Never removes — leaving an org just means the next pass
 * doesn't re-provision it (the grant side is fail-closed; deprovision is separate).
 */
export async function syncGithubMemberships(
	db: DojoClient,
	userId: string,
	orgLogins: string[]
): Promise<{ joined: string[] }> {
	if (orgLogins.length === 0) return { joined: [] };
	const keys = orgLogins.map((l) => `github/${l.toLowerCase()}`);

	// Which of those org dōjōs actually exist (never provision a phantom tenant).
	const { data: tenants, error } = await db.from('tenants').select('id, key').in('key', keys);
	if (error) throw new AdminError(500, error.message);

	// The caller's existing memberships (don't re-join).
	const { data: existing, error: e2 } = await db
		.from('memberships')
		.select('tenant_id')
		.eq('user_id', userId)
		.is('disabled_at', null);
	if (e2) throw new AdminError(500, e2.message);
	const have = new Set((existing ?? []).map((m) => (m as { tenant_id: string }).tenant_id));

	const joined: string[] = [];
	for (const t of (tenants ?? []) as { id: string; key: string }[]) {
		if (have.has(t.id)) continue;
		await addMember(db, t.id, {
			user_id: userId,
			kind: 'employer',
			authenticated_via: 'github_oauth',
			role: 'contributor'
		});
		joined.push(t.key);
	}
	return { joined };
}
