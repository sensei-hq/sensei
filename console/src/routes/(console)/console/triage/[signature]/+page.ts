import type { PageLoad } from './$types';
import { listTriage, TriageApiError, type TriageRow } from '$lib/triage-data';

// Candidate detail. The triage API has no single-row endpoint, so the row is
// found by signature within the tenant's open queue (list_triage). `row` is null
// when the signature isn't in the open queue (already decided, or unknown) — the
// screen shows a not-found state rather than crashing. Degrades to an error
// banner without a live dojo service (Jerry's live-verify step).
export const load: PageLoad = async ({ params, parent, fetch }) => {
	const { tenantKey } = await parent();
	const signature = params.signature;
	let row: TriageRow | null = null;
	let error: string | null = null;
	try {
		const queue = await listTriage(tenantKey, { fetch });
		row = queue.find((r) => r.signature === signature) ?? null;
	} catch (e) {
		error = e instanceof TriageApiError ? e.message : 'could not reach the dojo service';
	}
	return { signature, row, error };
};
