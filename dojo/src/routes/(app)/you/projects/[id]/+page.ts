import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { youHref } from '$lib/nav';
import { listUserProjects, getUserProjectConstitution } from '$lib/client-data';
import { toKitProjects } from '$lib/projects-map';
import { rulesToLadder, relayRules, relayToKitConflicts } from '$lib/constitution-map';
import type { KitProject, KitLadderRung, KitConflict } from '$lib/components/kit/types';

// The project-constitution drill-in loader (F4). Resolves the REAL project from
// the user-wide read (GET /v1/you/projects) by id, then reads its daemon-resolved
// constitution (GET /v1/you/projects/{slug}/constitution) and maps it to the kit
// ladder + conflicts the preview renders. The daemon OWNS the resolution; the dōjō
// only displays it (never re-resolves). No federated constitution yet → an empty
// ladder, and the screen shows its honest "resolves in your editor" state — never
// a fabricated ladder. A no-membership viewer, an unknown id, or a projects-read
// error redirects to the list so the drill never dead-ends on a fabricated project.
export const load: PageLoad = async ({ params, parent, fetch }) => {
	const { hasMembership, accessToken } = await parent();
	if (!hasMembership) redirect(307, youHref('projects'));

	let project: KitProject | undefined;
	try {
		const projects = toKitProjects(await listUserProjects({ fetch, accessToken }));
		project = projects.find((p) => p.id === params.id);
	} catch {
		redirect(307, youHref('projects'));
	}
	if (!project) redirect(307, youHref('projects'));

	// The real resolved constitution. Best-effort: a read error just leaves the
	// ladder empty (honest "resolves in your editor" state), never fails the drill
	// — the real project header still renders.
	let ladder: KitLadderRung[] = [];
	let conflicts: KitConflict[] = [];
	try {
		const c = await getUserProjectConstitution(project.repo, { fetch, accessToken });
		if (c) {
			ladder = rulesToLadder(relayRules(c));
			conflicts = relayToKitConflicts(c);
		}
	} catch {
		// honest-empty — the screen degrades to "resolves in your editor".
	}

	return { project, ladder, conflicts };
};
