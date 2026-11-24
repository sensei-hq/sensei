// GET /v1/you/graph/level0 — one branch's Code map L0 (#268).
//
// The dōjō's answer to the daemon's /api/workspaces/{id}/diagrams/codemap:
// modules as blocks over the caller's graph read, grouped by layer (default)
// or folders, narrowed to a focus before folding. The repository travels as
// its key and resolves caller-side (no IDOR on uuids); the graph tables' RLS
// decides what the read returns — a non-member reads an honest empty.
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
		const groupBy = url.searchParams.get('groupBy') ?? 'layers';
		if (groupBy !== 'layers' && groupBy !== 'folders') return apiError(400, 'groupBy is layers or folders');
		const focus = url.searchParams.get('focus');
		const segment = url.searchParams.get('segment');
		if (focus && segment) return apiError(400, 'focus and segment are exclusive');
		const db = callerDb(caller);
		const repositoryId = await readableRepository(db, repoKey);
		const port = createGraphPort(() => readBranchGraph(db, repositoryId, branch));
		return Response.json(
			await port.level0(
				groupBy,
				focus ? { kind: 'module', id: focus } : segment ? { kind: 'segment', id: segment } : null
			)
		);
	} catch (e) {
		return failure(e);
	}
};
