import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { ORG_SECTIONS, labelForSection } from '$lib/dojo2-nav';
import { orgBySlug } from '$lib/dojo2-chrome';
import { tabForSection } from '$lib/dojo2-role-surfaces-view';
import { guardTenantScope } from '$lib/org-guard';
import { listEngagements, DojoApiError, type Engagement } from '$lib/client-data';
import { toKitEngagements } from '$lib/dojo2-client-map';
import type { KitEngagement } from '$lib/components/kit/types';
import {
	orgProjectsFor,
	orgConstitutionFor,
	triageGroupsFor,
	candidateDetailFor,
	approvalsFor,
	knowledgeFor,
	confidentialityFor,
	incidentsFor,
	clientAuditFor,
	membersFor,
	rolePoliciesFor,
	auditLogFor,
	scopeOwnersFor,
	identityFor,
	healthFor,
	billingFor
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
// The lead Clients `engagements` section now loads REAL `/v1` data via the SAME
// console client the shipped `(console)` engagements screen uses
// (client-data.listEngagements → GET /v1/t/{tenant}/engagements, LEAD floor),
// mapped to the kit shape by the pure `toKitEngagements`. The fetch runs behind
// `guardTenantScope` on the resolved org's tenant key (`org.url`) and degrades to
// an empty list on a live-service failure / non-lead 403 / local `/v1` 404 so the
// screen still renders. The other sections still render off the kit fixtures
// (presentational): the Overview `ladder`/`projects`, the rest of the maintainer
// Govern consoles (triage/approvals/knowledge), the rest of the lead Clients
// consoles (confidentiality — its route isn't built yet — incidents/clientaudit)
// and the admin consoles (members/scopes/identity/audit/health/billing). The
// `members` and `audit` sections are the SAME screen (ScrRoleSurfaces); the loader
// maps the section to its opening tab via `tabForSection`.
export const load: PageLoad = async ({ parent, params, fetch }) => {
	const { memberships, accessToken } = await parent();
	const org = orgBySlug(memberships, params.slug);
	if (!org) redirect(307, '/you');
	if (!ORG_SECTIONS.includes(params.section)) redirect(307, `/org/${params.slug}`);
	const slug = params.slug;

	// Engagements: real /v1 data for the lead Clients register (only fetched for
	// that section). The tenant key is the resolved org's `url`; a fetch failure
	// (or a non-lead 403 / dev-only 404) degrades to an empty register + a
	// surfaced error so the screen renders.
	let engagements: KitEngagement[] = [];
	let engagementsError: string | null = null;
	if (params.section === 'engagements') {
		try {
			const guarded = await guardTenantScope<Engagement[]>(org.url, [], (tk) =>
				listEngagements(tk, { fetch, accessToken })
			);
			engagements = toKitEngagements(guarded.value);
		} catch (e) {
			engagementsError = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
		}
	}

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
		// Lead Clients consoles. Engagements = real /v1; the confidentiality panel
		// stays on the fixture (its route isn't built yet).
		engagements,
		engagementsError,
		confidentiality: confidentialityFor(slug),
		incidents: incidentsFor(slug),
		clientAudit: clientAuditFor(slug),
		// Admin consoles. `roleTab` is the opening tab of the shared role-surfaces
		// screen — `members` → Members, `audit` → Audit.
		roleTab: tabForSection(params.section),
		members: membersFor(slug),
		rolePolicies: rolePoliciesFor(slug),
		auditLog: auditLogFor(slug),
		scopeOwners: scopeOwnersFor(slug),
		identity: identityFor(slug),
		health: healthFor(slug),
		billing: billingFor(slug)
	};
};
