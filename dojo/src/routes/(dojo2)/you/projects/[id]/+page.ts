import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { youHref } from '$lib/dojo2-nav';
import { projects, ladder, conflicts } from '$lib/components/kit/fixtures';

// The project-constitution preview drill-in loader (mockup ScrProjectPreview).
// Resolves the project by id from the kit fixtures (presentational — real `/v1`
// wiring is a later chunk) and supplies the shared ladder + conflicts the
// resolution reads. A membership-less viewer has no projects, and an unknown id
// has nothing to preview — both redirect back to the projects list so the drill
// never dead-ends on a fabricated project.
export const load: PageLoad = async ({ params, parent }) => {
	const { hasMembership } = await parent();
	const project = hasMembership ? projects.find((p) => p.id === params.id) : undefined;
	if (!project) redirect(307, youHref('projects'));
	return { project, ladder, conflicts };
};
