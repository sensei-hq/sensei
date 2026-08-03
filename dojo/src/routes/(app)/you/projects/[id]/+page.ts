import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { youHref } from '$lib/nav';
import { listUserProjects } from '$lib/client-data';
import { toKitProjects } from '$lib/projects-map';
import type { KitProject } from '$lib/components/kit/types';

// The project-constitution drill-in loader. Resolves the REAL project from the
// user-wide read (F1's GET /v1/you/projects) by id — never a fixture, never a
// fabricated project. The composed constitution ladder is resolved by the daemon
// and NOT yet federated to the dōjō, so the screen renders an honest "resolves in
// your editor" state (empty ladder) rather than a fabricated one; F4 wires the
// real GET /v1/you/projects/{slug}/constitution. A no-membership viewer, an
// unknown id, or a read error redirects to the projects list so the drill never
// dead-ends on a fabricated project (the list surfaces a read error itself).
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

	// Honest-empty ladder/conflicts — the dōjō has no federated resolution yet
	// (F4). The screen shows a "resolves in your editor" state, not a fake ladder.
	return { project, ladder: [], conflicts: [] };
};
