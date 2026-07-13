import type { PageLoad } from './$types';
import { DojoApiError, listAudit, type AuditEvent } from '$lib/admin-data';

// Audit: the tenant's admin audit log, most recent first (list_audit, ADMIN
// floor). Degrades to an empty log + a surfaced error so the screen renders under
// SSR/prerender and without a live service / admin JWT (Jerry's live-verify step).
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let events: AuditEvent[] = [];
	let error: string | null = null;
	try {
		events = await listAudit(tenantKey, { fetch, accessToken, limit: 200 });
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { events, error };
};
