// GET /v1/you/graph/module — one module's files with its weighted, directed
// neighbours (#268). `id` is the folder; unknown here is the port's ok:false.
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
		const id = (url.searchParams.get('id') ?? '').trim();
		if (!repoKey || !branch || !id) return apiError(400, 'repo_key, branch and id are required');
		const db = callerDb(caller);
		const repositoryId = await readableRepository(db, repoKey);
		const port = createGraphPort(() => readBranchGraph(db, repositoryId, branch));
		return Response.json(await port.module(id));
	} catch (e) {
		return failure(e);
	}
};
