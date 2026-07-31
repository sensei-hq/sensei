import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { orgBySlug } from '$lib/chrome';
import { needsYou } from '$lib/components/kit/fixtures';
import { listProjects, DojoApiError } from '$lib/client-data';
import { toKitProjects } from '$lib/projects-map';
import type { KitProject } from '$lib/components/kit/types';

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
export const load: PageLoad = async ({ parent, params, fetch }) => {
	const { memberships, accessToken } = await parent();
	const org = orgBySlug(memberships, params.slug);
	if (!org) redirect(307, '/you');

	// The jurisdiction's projects — real `dojo.projects` (tenant-scoped), replacing
	// the orgProjectsFor(slug) fixture that showed fabricated repos to a real org
	// (F4 honesty). Honest-empty on a role-floor / dev / service failure — never a
	// fixture; degrades gracefully (the preview + count read 0).
	let projects: KitProject[] = [];
	let projectsError: string | null = null;
	try {
		projects = toKitProjects(await listProjects(org.url, { fetch, accessToken }));
	} catch (e) {
		projectsError = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	// The org "needs a maintainer" band — the first couple cross-dōjō items,
	// re-cast as this jurisdiction's queue (mockup `needsYou.slice(0, 2)`).
	const needs = needsYou.slice(0, 2);
	return {
		slug: params.slug,
		orgName: org.name,
		projects,
		projectsError,
		needs,
		stats: { members: org.members, needs: org.pending, projects: projects.length }
	};
};
