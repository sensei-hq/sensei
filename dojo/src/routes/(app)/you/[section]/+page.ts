import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { YOU_SECTIONS, labelForSection } from '$lib/nav';
import { toKitDojos } from '$lib/chrome';
import { listUserProjects, ClientApiError } from '$lib/client-data';
import { toKitProjects } from '$lib/projects-map';
import type { KitProject } from '$lib/components/kit/types';
import { stance, ladder, rulePacks } from '$lib/components/kit/fixtures';

// The personal-zone section loader — the non-inbox destinations (projects ·
// rules · packs · dojos · contributions). `dojos` binds REAL memberships (from
// the layout); `projects` binds the REAL user-wide `dojo.projects` read (F1,
// GET /v1/you/projects) — honest-empty on a genuine empty, an error STATE on a
// read failure (never []-as-success masking a failure). Contributions still has
// no backing route → honest empty (not a fabricated "helped 612"). Your own
// governance (stance · constitution · rule packs) stays fixture-backed pending
// its own route. An unknown/retired section redirects to the Inbox landing.
export const load: PageLoad = async ({ params, parent, fetch }) => {
	if (!YOU_SECTIONS.includes(params.section)) redirect(307, '/you');
	const { memberships, accessToken } = await parent();
	const section = params.section;

	// Real projects — only fetched on the projects section (no needless /v1 call
	// on the fixture-backed sections). The read is user-wide (across every dōjō).
	let projects: KitProject[] = [];
	let projectsError: string | null = null;
	if (section === 'projects') {
		try {
			projects = toKitProjects(await listUserProjects({ fetch, accessToken }));
		} catch (e) {
			projectsError = e instanceof ClientApiError ? e.message : 'could not reach the dojo service';
		}
	}

	return {
		section,
		title: labelForSection(section, 'you'),
		// Your own governance — always yours (still fixture-backed pending its route).
		stance,
		ladder,
		rulePacks,
		// Memberships — real, from the layout's `listUserOrgs`, mapped to KitDojo.
		dojos: toKitDojos(memberships),
		// Projects — real user-wide read; error ≠ empty (F1).
		projects,
		projectsError,
		// No backing route yet → honest empty, never fabricated (F4).
		contributionsMine: [],
		contributionsDownstream: [],
		contributionsStat: { approved: 0, pending: 0, helped: 0 }
	};
};
