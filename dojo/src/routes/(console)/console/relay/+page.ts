import type { PageLoad } from './$types';
import { listRuns, listGates, DojoApiError, type RelayRun, type RelayGate } from '$lib/relay-data';
import { guardTenantScope } from '$lib/org-guard';

// The relay run list: every supervised run the caller can see in this tenant
// (relay-data.listRuns → GET /v1/relay/session, Supabase-JWT plane) plus the pending
// gates awaiting the human (listGates → GET /v1/relay/gates) that surface as the
// "needs you" band above the list. A membership-less user (`tenantKey === null`,
// DJ1) skips both fetches via the shared guard → "join or create a Dōjō" empty
// state (the tenant-keyed relay plane can't serve a membership-free session
// tonight; the personal-Relay backend is deferred). For a member both are fetched
// in parallel; either failing surfaces the single `error` and degrades to empty
// lists so the screen still renders — mirrors triage/+page.ts.
type RelayLists = { runs: RelayRun[]; gates: RelayGate[] };
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let runs: RelayRun[] = [];
	let gates: RelayGate[] = [];
	let error: string | null = null;
	let noMembership = false;
	try {
		const guarded = await guardTenantScope<RelayLists>(
			tenantKey,
			{ runs: [], gates: [] },
			async (tk) => {
				const [r, g] = await Promise.all([
					listRuns(tk, { fetch, accessToken }),
					listGates(tk, { fetch, accessToken })
				]);
				return { runs: r, gates: g };
			}
		);
		runs = guarded.value.runs;
		gates = guarded.value.gates;
		noMembership = guarded.noMembership;
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { runs, gates, error, noMembership };
};
