// POST /v1/you/repositories — register the caller's shared repositories and map
// each to the tenant its forge org is connected to (§II.6, §VIII.1).
//
// User-scoped, not tenant-scoped, and deliberately: which tenant a repository
// belongs to is the question being asked, so it cannot also be the address.
//
// The daemon sends repositories it has ALREADY filtered by local intent — gate 1
// of the three, `sensei.repositories.visibility = 'shared'`, which is the
// daemon's alone and is never mirrored here. The dōjō never learns about a repo
// the user did not choose to share.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { registerRepositories, type RepoInput } from '$lib/server/repositories';

/** Accept only well-formed entries. A repo with no key has no identity, and
 *  registering it under a derived one would invent the very thing `repo_key`
 *  exists to make machine- and user-independent. */
function parseRepos(body: Record<string, unknown>): RepoInput[] {
	const raw = Array.isArray(body.repos) ? body.repos : null;
	if (!raw) throw new AdminError(400, 'repos must be an array');
	if (raw.length > 1000) throw new AdminError(400, 'too many repositories in one call (max 1000)');

	const out: RepoInput[] = [];
	for (const entry of raw) {
		const r = (entry ?? {}) as Record<string, unknown>;
		const key = typeof r.repo_key === 'string' ? r.repo_key.trim() : '';
		if (!key) continue;
		out.push({
			repo_key: key,
			remote_url: typeof r.remote_url === 'string' ? r.remote_url : null,
			name: typeof r.name === 'string' ? r.name : null
		});
	}
	return out;
}

export const POST: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, db } = await resolveCaller(request, locals);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const result = await registerRepositories(db, userId, parseRepos(body));
		return Response.json(result);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
