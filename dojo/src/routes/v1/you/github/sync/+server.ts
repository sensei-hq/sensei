// POST /v1/you/github/sync — auto-join the caller into the github/{org} dōjōs
// they belong to (F3c). Uses the caller's OWN GitHub OAuth token from their
// Supabase session (`session.provider_token`) to read their orgs, then provisions
// memberships for matching existing tenants (fail-closed — only proven, not-
// already-joined orgs). No GitHub token (not a GitHub sign-in) or a GitHub API
// hiccup → a best-effort no-op ({ synced:false }), never a 500 and never a
// fabricated membership. A DB error still fails closed.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { fetchGithubOrgLogins, syncGithubMemberships, AdminError } from '$lib/server/github-sync-data';

export const POST: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, db } = await resolveCaller(request, locals);
		const providerToken =
			(locals as { session?: { provider_token?: string | null } }).session?.provider_token ?? null;
		if (!providerToken) return Response.json({ joined: [], synced: false, reason: 'no_github_token' });

		let logins: string[];
		try {
			logins = await fetchGithubOrgLogins(providerToken);
		} catch {
			// Best-effort: a GitHub API hiccup shouldn't error the user's page.
			return Response.json({ joined: [], synced: false, reason: 'github_unreachable' });
		}
		const { joined } = await syncGithubMemberships(db, userId, logins);
		return Response.json({ joined, synced: true });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
