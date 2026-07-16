import type { PageLoad } from './$types';
import { listRuns, DojoApiError, type RelayRun } from '$lib/relay-data';

// The relay run list: every supervised run the caller can see in this tenant
// (relay-data.listRuns → GET /v1/relay/session, Supabase-JWT plane). Degrades to an
// empty list + a surfaced error so the screen renders under SSR/prerender and without
// a live dojo service (Jerry's live-verify step) — mirrors triage/+page.ts.
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let runs: RelayRun[] = [];
	let error: string | null = null;
	try {
		runs = await listRuns(tenantKey, { fetch, accessToken });
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { runs, error };
};
