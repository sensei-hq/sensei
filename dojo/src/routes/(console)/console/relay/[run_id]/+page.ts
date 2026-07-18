import type { PageLoad } from './$types';
import {
	getSegments,
	listRuns,
	listGates,
	DojoApiError,
	type RelaySegment,
	type RelayRun,
	type RelayGate
} from '$lib/relay-data';

// One run's detail: its segment outline (getSegments → GET /v1/relay/segments), the
// run header (found in listRuns — the personal beta has few runs; a dedicated getRun
// is a later optimization), and the tenant's pending gates filtered to THIS run
// (listGates → GET /v1/relay/gates) so a live "needs you" gate can be answered right
// here. Degrades to empty + a surfaced error so the shell renders without a live dojo
// service — mirrors the run-list load. A gates failure must not blank the page: the
// whole batch shares one catch and gates defaults to [].
export const load: PageLoad = async ({ parent, fetch, params }) => {
	const { tenantKey, accessToken } = await parent();
	const runId = params.run_id;
	let segments: RelaySegment[] = [];
	let run: RelayRun | null = null;
	let gates: RelayGate[] = [];
	let error: string | null = null;
	try {
		const [segs, runs, allGates] = await Promise.all([
			getSegments(tenantKey, runId, { fetch, accessToken }),
			listRuns(tenantKey, { fetch, accessToken }),
			listGates(tenantKey, { fetch, accessToken })
		]);
		segments = segs;
		run = runs.find((r) => r.run_id === runId) ?? null;
		gates = allGates.filter((g) => g.run_id === runId);
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { runId, run, segments, gates, error };
};
