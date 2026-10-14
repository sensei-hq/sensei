// /v1/you/graph — a repository's code graph, from the daemon (#225;
// docs/spec/dojo/graph-sync.md).
//
// POST files one batch: one repository, one commit of one tracked branch, a
// batch of file slices and — on the repository's last batch — its file list,
// so the dōjō prunes deleted files. GET reads back the content hash of every
// file the dōjō holds for one branch, which the daemon diffs against to push
// only what changed.
//
// As with metrics, the database re-decides everything on each write
// (dojo.file_graph_slices); this route checks the body's size before reading
// it and hands the rest to graph-ingest.ts.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { ingestGraph, graphHashes } from '$lib/server/graph-ingest';

/** Byte cap, checked BEFORE parsing — a body is a memory profile the client
 *  would otherwise choose. Larger than metrics' 2 MB: a slice carries a file's
 *  nodes and edges with their occurrences. The daemon batches whole files under
 *  it and counts a single file over it, never splitting one. */
const MAX_BYTES = 4 * 1024 * 1024;

function failure(e: unknown): Response {
	if (e instanceof Response) return e;
	if (e instanceof AdminError) return apiError(e.status, e.message);
	throw e;
}

export const POST: RequestHandler = async ({ request, locals }) => {
	try {
		const caller = await resolveCaller(request, locals);
		const declared = Number(request.headers.get('content-length') ?? '0');
		if (Number.isFinite(declared) && declared > MAX_BYTES) {
			return apiError(413, `body over ${MAX_BYTES} bytes`);
		}
		const body = await request.json().catch(() => null);
		return Response.json(await ingestGraph(callerDb(caller), body));
	} catch (e) {
		return failure(e);
	}
};

export const GET: RequestHandler = async ({ request, locals, url }) => {
	try {
		const caller = await resolveCaller(request, locals);
		const repoKey = url.searchParams.get('repo_key') ?? '';
		const branch = url.searchParams.get('branch') ?? '';
		return Response.json(await graphHashes(callerDb(caller), repoKey, branch));
	} catch (e) {
		return failure(e);
	}
};
