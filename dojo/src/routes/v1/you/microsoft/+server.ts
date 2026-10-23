// GET /v1/you/microsoft — whether the signed-in person's Microsoft account is
// linked to the dōjō (#308 R5c, D45): its Entra object id recorded as their
// azure_devops identity. A Microsoft sign-in that could not be linked FAILS
// CLOSED (no identity, no provisioning); this is how My dōjōs says so. The
// Microsoft account is Supabase's word (resolveCaller); who holds the identity
// is the system's read — another person's row is what a conflict is.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { microsoftLinkStatus } from '$lib/server/azure-signin';

export const GET: RequestHandler = async ({ request, locals }) => {
	try {
		const caller = await resolveCaller(request, locals);
		return Response.json(await microsoftLinkStatus(dojoDb(), caller.userId, caller.microsoft));
	} catch (e) {
		if (e instanceof Response) return e;
		const status = (e as { status?: unknown }).status;
		return apiError(typeof status === 'number' ? status : 500, e instanceof Error ? e.message : 'the read failed');
	}
};
