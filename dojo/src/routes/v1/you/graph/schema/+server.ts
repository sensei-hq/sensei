// GET /v1/you/graph/schema — one branch's schema as dbd's SchemaModel v3
// (#268), for the rokkit ER views. `enum_values` is not synced, so enums
// assemble named but valueless — the reads say so, and the route supplies
// the named empty rather than letting assemble guess.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { readableRepository, readBranchSchema } from '$lib/server/graph-reads';
import { assembleSchemaModel } from '$lib/server/graph-schema';

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
		const { entities, deps } = await readBranchSchema(db, repositoryId, branch);
		return Response.json(
			assembleSchemaModel(
				entities.map((e) => ({ ...e, enum_values: [] as string[] })),
				deps
			)
		);
	} catch (e) {
		return failure(e);
	}
};
