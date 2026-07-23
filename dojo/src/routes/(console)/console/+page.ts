import type { PageLoad } from './$types';
import { listTriage, TriageApiError, type TriageRow } from '$lib/triage-data';
import { guardTenantScope } from '$lib/org-guard';

// The console index. For a MEMBER it loads the tenant's open triage queue (the
// one live signal on the Overview — the metric cards / activity / published
// panels are the design's static demo content until those endpoints land).
// For a membership-less user (`tenantKey === null`, DJ1) the guard skips the
// fetch entirely (no `/v1/t/…` call) and the page renders the personal home
// instead — `noMembership` tells the page which branch to take. Failures for a
// member degrade to an empty queue + a surfaced error so the dashboard still
// renders under SSR/prerender and without a running dojo service.
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let error: string | null = null;
	let queue: TriageRow[] = [];
	let noMembership = false;
	try {
		const guarded = await guardTenantScope<TriageRow[]>(tenantKey, [], (tk) =>
			listTriage(tk, { fetch, accessToken })
		);
		queue = guarded.value;
		noMembership = guarded.noMembership;
	} catch (e) {
		error = e instanceof TriageApiError ? e.message : 'could not reach the dojo service';
	}
	return { queue, error, noMembership };
};
