import type { PageLoad } from './$types';
import { DojoApiError, listAudit, type AuditEvent } from '$lib/admin-data';
import { guardTenantScope } from '$lib/org-guard';

// Audit: the tenant's admin audit log, most recent first (list_audit, ADMIN
// floor). A membership-less user (`tenantKey === null`, DJ1) skips the fetch via
// the shared guard → "join or create a Dōjō" empty state. For a member it
// degrades to an empty log + a surfaced error so the screen renders under
// SSR/prerender and without a live service / admin JWT.
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let events: AuditEvent[] = [];
	let error: string | null = null;
	let noMembership = false;
	try {
		const guarded = await guardTenantScope<AuditEvent[]>(tenantKey, [], (tk) =>
			listAudit(tk, { fetch, accessToken, limit: 200 })
		);
		events = guarded.value;
		noMembership = guarded.noMembership;
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { events, error, noMembership };
};
