// GET /v1/you/graph/zones — Martin's abstractness against instability, one
// point per module (#268). Unplaceable points ride along as nulls (counted
// in coverage) so the screen can omit them honestly.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { readableRepository, readBranchGraph } from '$lib/server/graph-reads';
import { createGraphPort } from '$lib/server/graph-port';

function failure(e: unknown): Response {
	if (e instanceof Response) return e;
	if (e instanceof AdminError) return apiError(e.status, e.message);
	throw e;
}

export const GET: RequestHandler = async ({ request, locals, url }) => {
	try {
		const caller = await resolveCaller(request, locals);
		const repoKey = (url.searchParams.get('repo_key') ?? '').trim();
		const branch = (url.searchParams.get('branch') ?? '').trim();
		if (!repoKey || !branch) return apiError(400, 'repo_key and branch are required');
		const db = callerDb(caller);
		const repositoryId = await readableRepository(db, repoKey);
		const port = createGraphPort(() => readBranchGraph(db, repositoryId, branch));
		return Response.json(await port.zones());
	} catch (e) {
		return failure(e);
	}
};
