// PATCH /v1/you/repositories/election — choose whether a repository is shared.
//
// The write half of `dojo.all_my_repositories`. The view answers "may it, and
// did whoever holds authority choose it?"; this is where the choosing happens.
// Until this route existed nothing wrote `dojo.repository_elections` at all, so
// every user-authority repository read as `elected = false` forever.
//
// The repo key travels in the BODY, not the path. `github.com/owner/name`
// contains slashes, so as a path segment it needs encoding at every caller and
// decodes ambiguously against a `[...rest]` route — a key that round-trips
// wrong silently addresses a different repository, or none.
//
// User-scoped rather than tenant-scoped, matching `POST /v1/you/repositories`:
// which tenant a repository belongs to — and therefore which authority governs
// it — is derived, not supplied. See `$lib/server/elections`.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { setElection } from '$lib/server/elections';

export const PATCH: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, db } = await resolveCaller(request, locals);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;

		const repoKey = typeof body.repo_key === 'string' ? body.repo_key.trim() : '';
		if (!repoKey) return apiError(400, 'repo_key is required');

		// Strictly boolean. Coercing would make `{"elected":"false"}` — the shape a
		// form post or a shell client most easily produces — turn sharing ON, which
		// is the wrong direction to fail in for a disclosure toggle.
		if (typeof body.elected !== 'boolean') {
			return apiError(400, 'elected must be true or false');
		}

		return Response.json(await setElection(db, userId, repoKey, body.elected));
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
