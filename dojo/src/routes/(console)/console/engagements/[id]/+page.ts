import type { PageLoad } from './$types';
import {
	DojoApiError,
	listAuditArtifacts,
	listEngagements,
	type AuditArtifact,
	type Engagement
} from '$lib/client-data';
import { guardTenantScope } from '$lib/org-guard';

// One engagement's audit view (audit/artifacts, LEAD floor): every artifact
// shared under the engagement with its strip status, so the console can compute
// the done-gate (`non_dereferenced == count(dereferenced == false)`) and gate the
// compliance export. ALL rows are loaded (no `dereferenced` filter) so a
// non-dereferenced client-work artifact surfaces as a red-fail row.
//
// A membership-less user (`tenantKey === null`, DJ1) skips both fetches via the
// shared guard → the screen shows the "join or create a Dōjō" empty state. The
// engagement record is pulled from list_engagements (there's no single-GET on the
// backend) and matched by id — degrades to null + a surfaced error so the screen
// renders under SSR/prerender and without a live lead JWT.
type EngagementDetail = { engagements: Engagement[]; artifacts: AuditArtifact[] };
export const load: PageLoad = async ({ parent, params, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	const engagementId = params.id;
	let engagement: Engagement | null = null;
	let artifacts: AuditArtifact[] = [];
	let error: string | null = null;
	let noMembership = false;
	try {
		const guarded = await guardTenantScope<EngagementDetail>(
			tenantKey,
			{ engagements: [], artifacts: [] },
			async (tk) => {
				const [engagements, rows] = await Promise.all([
					listEngagements(tk, { fetch, accessToken }),
					listAuditArtifacts(tk, { fetch, accessToken, engagement: engagementId })
				]);
				return { engagements, artifacts: rows };
			}
		);
		engagement = guarded.value.engagements.find((e) => e.id === engagementId) ?? null;
		artifacts = guarded.value.artifacts;
		noMembership = guarded.noMembership;
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { engagementId, engagement, artifacts, error, noMembership };
};
