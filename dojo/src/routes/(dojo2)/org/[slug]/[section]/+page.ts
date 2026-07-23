import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { ORG_SECTIONS, labelForSection } from '$lib/dojo2-nav';
import { orgBySlug } from '$lib/dojo2-chrome';
import { orgProjectsFor, orgConstitutionFor } from '$lib/components/kit/fixtures';

// An org-zone section route. Resolves the org from the slug against real
// memberships (redirect to /you if not a member) and validates the section
// against the known org destinations (redirect to the org home for an unknown
// tail) — the two-nav never dead-ends.
//
// The ported Overview sections render off the kit fixtures (presentational —
// real `/v1` wiring is a later chunk): `ladder` supplies the dōjō's authored
// constitution sections, `projects` the jurisdiction project list. Any remaining
// section still renders the "coming in the rebuild" placeholder.
export const load: PageLoad = async ({ parent, params }) => {
	const { memberships } = await parent();
	const org = orgBySlug(memberships, params.slug);
	if (!org) redirect(307, '/you');
	if (!ORG_SECTIONS.includes(params.section)) redirect(307, `/org/${params.slug}`);
	return {
		slug: params.slug,
		orgName: org.name,
		section: params.section,
		title: labelForSection(params.section, 'org'),
		// Overview-section data (ported fixtures this chunk).
		sections: orgConstitutionFor(params.slug),
		projects: orgProjectsFor(params.slug)
	};
};
