import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { ORG_SECTIONS, labelForSection } from '$lib/dojo2-nav';
import { orgBySlug } from '$lib/dojo2-chrome';
import {
	orgProjectsFor,
	orgConstitutionFor,
	triageGroupsFor,
	candidateDetailFor,
	approvalsFor,
	knowledgeFor,
	engagementsFor,
	confidentialityFor,
	incidentsFor,
	clientAuditFor
} from '$lib/components/kit/fixtures';

// An org-zone section route. Resolves the org from the slug against real
// memberships (redirect to /you if not a member) and validates the section
// against the known org destinations (redirect to the org home for an unknown
// tail) — the two-nav never dead-ends. Whether a role-scoped section is even
// reachable is enforced by `navForOrg` in the shell (a viewer only sees the nav
// their rank unlocks); a hand-typed URL to a section above your rank still
// renders its screen off fixtures this chunk (real `/v1` authorization lands
// with the wiring).
//
// The ported sections render off the kit fixtures (presentational): the
// Overview `ladder`/`projects`, the maintainer Govern consoles
// (triage/approvals/knowledge) and the lead Clients consoles
// (engagements/incidents/clientaudit). Any remaining section still renders the
// "coming in the rebuild" placeholder.
export const load: PageLoad = async ({ parent, params }) => {
	const { memberships } = await parent();
	const org = orgBySlug(memberships, params.slug);
	if (!org) redirect(307, '/you');
	if (!ORG_SECTIONS.includes(params.section)) redirect(307, `/org/${params.slug}`);
	const slug = params.slug;
	return {
		slug,
		orgName: org.name,
		section: params.section,
		title: labelForSection(params.section, 'org'),
		// Overview-section data (ported fixtures this chunk).
		sections: orgConstitutionFor(slug),
		projects: orgProjectsFor(slug),
		// Maintainer Govern consoles.
		triage: triageGroupsFor(slug),
		candidateDetail: candidateDetailFor(slug),
		approvals: approvalsFor(slug),
		knowledge: knowledgeFor(slug),
		// Lead Clients consoles.
		engagements: engagementsFor(slug),
		confidentiality: confidentialityFor(slug),
		incidents: incidentsFor(slug),
		clientAudit: clientAuditFor(slug)
	};
};
