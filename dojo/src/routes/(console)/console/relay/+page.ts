import type { PageLoad } from './$types';
import { listRuns, listGates, DojoApiError, type RelayRun, type RelayGate } from '$lib/relay-data';

// The relay run list: every supervised run the caller can see in this tenant
// (relay-data.listRuns → GET /v1/relay/session, Supabase-JWT plane) plus the pending
// gates awaiting the human (listGates → GET /v1/relay/gates) that surface as the
// "needs you" band above the list. Both are fetched in parallel; either failing
// surfaces the single `error` string and degrades to empty lists so the screen still
// renders under SSR/prerender and without a live dojo service — mirrors triage/+page.ts.
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let runs: RelayRun[] = [];
	let gates: RelayGate[] = [];
	let error: string | null = null;
	try {
		[runs, gates] = await Promise.all([
			listRuns(tenantKey, { fetch, accessToken }),
			listGates(tenantKey, { fetch, accessToken })
		]);
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { runs, gates, error };
};
