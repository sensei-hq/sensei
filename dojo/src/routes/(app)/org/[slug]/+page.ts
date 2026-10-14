import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { orgBySlug } from '$lib/chrome';
import { listTriage } from '$lib/triage-data';
import { toKitNeeds } from '$lib/triage-map';
import { rankOf, K2_ROLE_RANK } from '$lib/nav';
import type { KitNeed } from '$lib/components/kit/types';
import { listProjects, DojoApiError } from '$lib/client-data';
import { toKitProjects } from '$lib/projects-map';
import type { KitProject } from '$lib/components/kit/types';

// The org home resolves its org from the URL slug against the caller's real
// memberships (not just the tenant cookie), so a direct link to /org/{slug}
// shows the right dōjō. A slug the caller isn't a member of redirects to the
// personal landing — never a fabricated org (DJ1).
//
// Everything here is real: the jurisdiction's projects, the "needs a maintainer"
// band (the tenant's open triage queue, maintainer floor — #241), and the stat
// row drawn from the resolved membership (members · pending · project count).
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
	// The "needs a maintainer" band: the open triage queue's first items. Below
	// maintainer it is not the viewer's queue — nothing is asked, nothing shown.
	// A failed read is an error, never an empty or invented band.
	let needs: KitNeed[] = [];
	let needsError: string | null = null;
	if (rankOf(org.role) >= K2_ROLE_RANK.maintainer) {
		try {
			needs = toKitNeeds(await listTriage(org.url, { fetch, accessToken }), org.name);
		} catch (e) {
			needsError = e instanceof Error && e.message ? e.message : 'could not read the triage queue';
		}
	}
	return {
		slug: params.slug,
		orgName: org.name,
		projects,
		projectsError,
		needs,
		needsError,
		stats: { members: org.members, needs: org.pending, projects: projects.length }
	};
};
