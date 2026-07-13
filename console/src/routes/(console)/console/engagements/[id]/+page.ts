import type { PageLoad } from './$types';
import {
	DojoApiError,
	listAuditArtifacts,
	listEngagements,
	type AuditArtifact,
	type Engagement
} from '$lib/client-data';

// One engagement's audit view (audit/artifacts, LEAD floor): every artifact
// shared under the engagement with its strip status, so the console can compute
// the done-gate (`non_dereferenced == count(dereferenced == false)`) and gate the
// compliance export. ALL rows are loaded (no `dereferenced` filter) so a
// non-dereferenced client-work artifact surfaces as a red-fail row.
//
// The engagement record is pulled from list_engagements (there's no single-GET on
// the backend) and matched by id — degrades to null + a surfaced error so the
// screen renders under SSR/prerender and without a live client-lead JWT.
export const load: PageLoad = async ({ parent, params, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	const engagementId = params.id;
	let engagement: Engagement | null = null;
	let artifacts: AuditArtifact[] = [];
	let error: string | null = null;
	try {
		const [engagements, rows] = await Promise.all([
			listEngagements(tenantKey, { fetch, accessToken }),
			listAuditArtifacts(tenantKey, { fetch, accessToken, engagement: engagementId })
		]);
		engagement = engagements.find((e) => e.id === engagementId) ?? null;
		artifacts = rows;
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { engagementId, engagement, artifacts, error };
};
