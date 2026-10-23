// GET  /v1/t/{origin}/{org}/teams — the dōjō's teams with their members (#307 A3,
//      D7). Any member reads them (RLS: dojo.is_member); the members are named
//      through the service client, since a non-admin may read a team's ids but not
//      the identities (the access matrix's resolveDisplayNames exception).
// POST /v1/t/{origin}/{org}/teams {name} — create one (admins). 201.
// A team is a grouping for reports and bulk viewer grants; it grants no access.
import { ACCESS } from '$lib/server/dojo-auth';
import { dojoDb } from '$lib/server/dojo-supabase';
import { tenantRoute, nameFrom } from '$lib/server/tenant-route';
import { listTeams, createTeam } from '$lib/server/teams-data';

export const GET = tenantRoute(ACCESS.member, async (db, caller) =>
	Response.json({ teams: await listTeams(db, caller.tenantId, dojoDb()) })
);

export const POST = tenantRoute(ACCESS.admin, async (db, caller, { request }) => {
	await createTeam(db, caller.tenantId, await nameFrom(request));
	return new Response(null, { status: 201 });
});
