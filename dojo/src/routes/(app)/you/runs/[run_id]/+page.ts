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
import { guardTenantScope } from '$lib/org-guard';

// One personal run's detail — the drill-in behind /you/runs. Ports the shipped
// (console) relay run loader: its segment outline (getSegments → GET /v1/relay/
// segments), the run header (found in listRuns — the personal beta has few runs; a
// dedicated getRun is a later optimization), and the tenant's pending gates filtered
// to THIS run (listGates → GET /v1/relay/gates) so a live "needs you" gate can be
// answered right here. tenantKey/accessToken come from the (app) layout via
// parent() — the SAME shared console context the (console) shell resolves.
//
// A membership-less user (`tenantKey === null`, DJ1) skips all three fetches via the
// shared guard → the screen shows the join-or-create empty state. Otherwise it
// degrades to empty + a surfaced error so the shell renders without a live dojo
// service. A gates failure must not blank the page: the whole batch shares one catch
// and gates defaults to [].
type RunDetail = { segments: RelaySegment[]; runs: RelayRun[]; gates: RelayGate[] };
export const load: PageLoad = async ({ parent, fetch, params }) => {
	const { tenantKey, accessToken } = await parent();
	const runId = params.run_id;
	let segments: RelaySegment[] = [];
	let run: RelayRun | null = null;
	let gates: RelayGate[] = [];
	let error: string | null = null;
	let noMembership = false;
	try {
		const guarded = await guardTenantScope<RunDetail>(
			tenantKey,
			{ segments: [], runs: [], gates: [] },
			async (tk) => {
				const [segs, runs, allGates] = await Promise.all([
					getSegments(tk, runId, { fetch, accessToken }),
					listRuns(tk, { fetch, accessToken }),
					listGates(tk, { fetch, accessToken })
				]);
				return { segments: segs, runs, gates: allGates };
			}
		);
		segments = guarded.value.segments;
		run = guarded.value.runs.find((r) => r.run_id === runId) ?? null;
		gates = guarded.value.gates.filter((g) => g.run_id === runId);
		noMembership = guarded.noMembership;
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { runId, run, segments, gates, error, noMembership };
};
