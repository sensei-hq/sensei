import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { orgBySlug } from '$lib/dojo2-chrome';
import { orgProjectsFor, needsYou } from '$lib/components/kit/fixtures';

// The org home resolves its org from the URL slug against the caller's real
// memberships (not just the tenant cookie), so a direct link to /org/{slug}
// shows the right dōjō. A slug the caller isn't a member of redirects to the
// personal landing — never a fabricated org (DJ1).
//
// The projects-in-jurisdiction, org needs band, and jurisdiction stat row render
// off the ported kit fixtures (presentational — real `/v1` wiring is a later
// chunk). The stat row is drawn from the resolved membership (members · pending →
// "need a maintainer" · the jurisdiction project count) so it stays honest to the
// org the caller actually belongs to.
export const load: PageLoad = async ({ parent, params }) => {
	const { memberships } = await parent();
	const org = orgBySlug(memberships, params.slug);
	if (!org) redirect(307, '/you');

	const projects = orgProjectsFor(params.slug);
	// The org "needs a maintainer" band — the first couple cross-dōjō items,
	// re-cast as this jurisdiction's queue (mockup `needsYou.slice(0, 2)`).
	const needs = needsYou.slice(0, 2);
	return {
		slug: params.slug,
		orgName: org.name,
		projects,
		needs,
		stats: { members: org.members, needs: org.pending, projects: projects.length }
	};
};
