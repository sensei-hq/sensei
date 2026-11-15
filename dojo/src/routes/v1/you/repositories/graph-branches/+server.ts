// PUT/DELETE /v1/you/repositories/graph-branches — track or untrack a branch of
// a repository's shared code graph.
//
// The write half of the plan's `graph_branches`: the daemon pushes a checkout
// only when its branch is tracked here, and tracking none means the graph does
// not sync — never a guessed `main`. Untracking deletes that branch's graph,
// so the screen confirms in-page first; this route just records the decision.
//
// The repo key travels in the BODY, not the path, as the election route's
// comment explains: a key round-tripped wrong silently addresses a different
// repository, or none.
//
// User-scoped rather than tenant-scoped, matching the election route: which
// tenant a repository belongs to — and therefore who may configure it — is
// derived, not supplied. See `$lib/server/repositories`.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { setGraphBranch } from '$lib/server/repositories';
import { GRAPH_LABELS } from '$lib/graph-labels';

async function branchWrite(
	request: Request,
	locals: App.Locals,
	tracked: boolean
): Promise<Response> {
	try {
		const caller = await resolveCaller(request, locals);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;

		const repoKey = typeof body.repo_key === 'string' ? body.repo_key.trim() : '';
		if (!repoKey) return apiError(400, 'repo_key is required');

		// Refused here as a 400 rather than by the database: the function's
		// invalid_parameter_value would surface as a 422, and a nameless branch
		// is a bad request, not an unprocessable one.
		const branch = typeof body.branch === 'string' ? body.branch.trim() : '';
		if (!branch) return apiError(400, 'branch is required');

		let label: string | null = null;
		if (body.label !== undefined && body.label !== null) {
			if (typeof body.label !== 'string') return apiError(400, 'label must be a string');
			const trimmed = body.label.trim();
			if (trimmed && !(GRAPH_LABELS as readonly string[]).includes(trimmed)) {
				return apiError(400, `label must be one of ${GRAPH_LABELS.join(', ')}`);
			}
			label = trimmed || null;
		}

		return Response.json(
			await setGraphBranch(callerDb(caller), repoKey, branch, tracked, tracked ? label : null)
		);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
}

export const PUT: RequestHandler = async ({ request, locals }) => branchWrite(request, locals, true);

export const DELETE: RequestHandler = async ({ request, locals }) =>
	branchWrite(request, locals, false);
