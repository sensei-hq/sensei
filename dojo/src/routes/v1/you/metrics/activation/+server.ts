// PATCH /v1/you/metrics/activation — switch a catalogue metric off (or back on)
// for one repository.
//
// The write half of `dojo.metric_activations`. Until this existed the table was
// inert: its comment promised that disabling a metric "STOPS ITS COMPUTATION",
// and nothing anywhere read OR wrote it, so the promise was undischarged in both
// directions.
//
// The repo key travels in the BODY, not the path: `github.com/owner/name`
// contains slashes, so as a path segment it needs encoding at every caller and
// decodes ambiguously against a `[...rest]` route — a key that round-trips wrong
// silently addresses a different repository, or none. Same reasoning as the
// election route.
//
// User-scoped rather than tenant-scoped: which tenant owns the repository, and
// therefore whose ruling this is, is DERIVED from `dojo.all_my_repositories`, not
// supplied. A body that could name the tenant would let a member of one dōjō
// write another's cost decision.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { setMetricActivation } from '$lib/server/metric-activation';

export const PATCH: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, db } = await resolveCaller(request, locals);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;

		const repoKey = typeof body.repo_key === 'string' ? body.repo_key.trim() : '';
		if (!repoKey) return apiError(400, 'repo_key is required');

		const metric = typeof body.metric === 'string' ? body.metric.trim() : '';
		if (!metric) return apiError(400, 'metric is required');

		// Strictly boolean. Coercing would make `{"enabled":"false"}` — the shape a
		// form post or a shell client most easily produces — turn a metric ON,
		// which is the wrong direction to fail for a cost lever: it silently
		// resumes work the tenant is paying to avoid.
		if (typeof body.enabled !== 'boolean') {
			return apiError(400, 'enabled must be true or false');
		}

		return Response.json(await setMetricActivation(db, userId, repoKey, metric, body.enabled));
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
