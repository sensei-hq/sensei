import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { orgHref } from '$lib/nav';
import { orgBySlug } from '$lib/chrome';
import { orgProjectsFor, ladder, conflicts } from '$lib/components/kit/fixtures';

// The org project-constitution preview drill-in loader (mockup ScrProjectPreview
// in the org context). Resolves the org from the slug against real memberships
// (redirect to /you if not a member), then the project by id from the org's
// jurisdiction fixtures (presentational — real `/v1` wiring is a later chunk),
// and supplies the shared ladder + conflicts the resolution reads. The
// resolution engine is classification-driven and shared, so an org (company)
// project layers the company baseline. An unknown id has nothing to preview and
// redirects back to the org project list so the drill never dead-ends.
export const load: PageLoad = async ({ parent, params }) => {
	const { memberships } = await parent();
	const org = orgBySlug(memberships, params.slug);
	if (!org) redirect(307, '/you');

	const project = orgProjectsFor(params.slug).find((p) => p.id === params.id);
	if (!project) redirect(307, orgHref(params.slug, 'projects'));
	return { slug: params.slug, orgName: org.name, project, ladder, conflicts };
};
