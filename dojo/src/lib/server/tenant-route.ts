// The shape every tenant-addressed route shares (#307 A3): resolve the caller at
// a floor, run the work with the caller's own client (so RLS and the database's
// functions decide), and turn a refusal into its status with the reason. A
// Response thrown on the way (no session, below the floor) is returned as is.
import type { RequestEvent } from '@sveltejs/kit';
import { callerDb } from './caller-db';
import { resolveTenantAccess, apiError, type Caller } from './dojo-auth';
import { AdminError, type DojoClient } from './admin-data';

type TenantEvent = RequestEvent<Partial<Record<string, string>> & { origin: string; org: string }>;
type Floor = Parameters<typeof resolveTenantAccess>[4];

export function tenantRoute(
	floor: Floor,
	work: (db: DojoClient, caller: Caller, event: TenantEvent) => Promise<Response>
) {
	return async (event: TenantEvent): Promise<Response> => {
		try {
			const { params, request, locals } = event;
			const caller = await resolveTenantAccess(params.origin, params.org, request, locals, floor);
			return await work(callerDb(caller), caller, event);
		} catch (e) {
			if (e instanceof Response) return e;
			if (e instanceof AdminError) return apiError(e.status, e.message);
			throw e;
		}
	};
}

/** 204 — a write that answers with nothing. */
export const noContent = () => new Response(null, { status: 204 });

/** The `name` of a JSON body, or a 400. */
export async function nameFrom(request: Request): Promise<string> {
	const body = (await request.json().catch(() => ({}))) as { name?: unknown };
	if (typeof body.name !== 'string' || !body.name.trim()) throw new AdminError(400, 'a name is required');
	return body.name;
}
