import type { PageLoad } from './$types';
import { getSegments, listRuns, DojoApiError, type RelaySegment, type RelayRun } from '$lib/relay-data';

// One run's detail: its segment outline (getSegments → GET /v1/relay/segments) plus
// the run header (found in listRuns — the personal beta has few runs; a dedicated
// getRun is a later optimization). Degrades to empty + a surfaced error so the shell
// renders without a live dojo service — mirrors the run-list load.
export const load: PageLoad = async ({ parent, fetch, params }) => {
	const { tenantKey, accessToken } = await parent();
	const runId = params.run_id;
	let segments: RelaySegment[] = [];
	let run: RelayRun | null = null;
	let error: string | null = null;
	try {
		const [segs, runs] = await Promise.all([
			getSegments(tenantKey, runId, { fetch, accessToken }),
			listRuns(tenantKey, { fetch, accessToken })
		]);
		segments = segs;
		run = runs.find((r) => r.run_id === runId) ?? null;
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { runId, run, segments, error };
};
