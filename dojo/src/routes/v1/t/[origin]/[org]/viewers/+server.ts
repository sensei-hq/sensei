// GET /v1/t/{origin}/{org}/viewers — the dōjō's viewers (#307 A3, D8): people
// invited in as viewers, not members, each with the workspaces granted to them.
// ADMIN floor; read with the caller's own JWT so RLS decides. Fails closed.
import { ACCESS } from '$lib/server/dojo-auth';
import { tenantRoute } from '$lib/server/tenant-route';
import { listDojoViewers } from '$lib/server/viewers-data';

export const GET = tenantRoute(ACCESS.admin, async (db, caller) =>
	Response.json({ viewers: await listDojoViewers(db, caller.tenantId) })
);
