import type { PageLoad } from './$types';
import { listTriage, TriageApiError, type TriageRow } from '$lib/triage-data';
import { guardTenantScope } from '$lib/org-guard';

// Candidate detail. The triage API has no single-row endpoint, so the row is
// found by signature within the tenant's open queue (list_triage). A
// membership-less user (`tenantKey === null`, DJ1) skips the fetch via the shared
// guard → the screen shows the "join or create a Dōjō" empty state. Otherwise
// `row` is null when the signature isn't in the open queue (already decided, or
// unknown) — the screen shows a not-found state rather than crashing. Degrades to
// an error banner without a live dojo service.
export const load: PageLoad = async ({ params, parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	const signature = params.signature;
	let row: TriageRow | null = null;
	let error: string | null = null;
	let noMembership = false;
	try {
		const guarded = await guardTenantScope<TriageRow[]>(tenantKey, [], (tk) =>
			listTriage(tk, { fetch, accessToken })
		);
		row = guarded.value.find((r) => r.signature === signature) ?? null;
		noMembership = guarded.noMembership;
	} catch (e) {
		error = e instanceof TriageApiError ? e.message : 'could not reach the dojo service';
	}
	return { signature, row, error, noMembership };
};
