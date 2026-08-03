// GET /v1/you/contributions — the caller's contributions across EVERY dōjō (F5):
// { mine } = artifacts they contributed, { downstream } = the distribution-ledger
// items approved for them. User-wide (no tenant, no role floor): resolved from the
// JWT, authorized by contributed_by = caller + membership ownership. Read-only;
// fails closed; honest-empty until the contribute pipeline federates. Never a fixture.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import {
	userMembershipIds,
	listUserContributions,
	listUserDownstream,
	AdminError
} from '$lib/server/contributions-data';

export const GET: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, db } = await resolveCaller(request, locals);
		const [mine, membershipIds] = await Promise.all([
			listUserContributions(db, userId),
			userMembershipIds(db, userId)
		]);
		const downstream = await listUserDownstream(db, membershipIds);
		return Response.json({ mine, downstream });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
