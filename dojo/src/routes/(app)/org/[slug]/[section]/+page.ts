import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { ORG_SECTIONS, labelForSection } from '$lib/nav';
import { orgBySlug } from '$lib/chrome';
import { tabForSection } from '$lib/role-surfaces-view';
import { guardTenantScope } from '$lib/org-guard';
import {
	listEngagements,
	listIncidents,
	getKnowledge,
	listClientAuditLedger,
	listProjects,
	DojoApiError,
	type Engagement,
	type Incident,
	type KnowledgeLibraryWire,
	type ClientAuditEntry,
	type ProjectRow
} from '$lib/client-data';
import { listTriage, type TriageRow } from '$lib/triage-data';
import {
	listMembers,
	listPolicies,
	listIdentities,
	listAudit,
	getHealth,
	getBilling,
	type Membership,
	type Policy,
	type Identity,
	type AuditEvent,
	type HealthRollup,
	type BillingResponse
} from '$lib/admin-data';
import { toKitBilling } from '$lib/billing-map';
import { toKitEngagements } from '$lib/client-map';
import {
	toKitTriageGroups,
	toKitApprovals,
	toKitCandidateDetail
} from '$lib/triage-map';
import { flattenCandidates } from '$lib/triage/candidates-view';
import {
	toKitMembers,
	toKitRolePolicies,
	toKitAuditThread,
	toKitIdentity,
	toKitHealth
} from '$lib/admin-map';
import { toKitIncidents, toKitClientAuditLedger } from '$lib/incidents-map';
import { toKitKnowledge } from '$lib/knowledge-map';
import { toKitProjects } from '$lib/projects-map';
import type {
	KitEngagement,
	KitTriageGroup,
	KitApproval,
	KitCandidateDetail,
	KitMember,
	KitRolePolicy,
	KitChatTurn,
	KitKnowledge,
	KitProject,
	KitIdentity,
	KitIncident,
	KitClientAuditRow,
	KitHealth,
	KitBilling
} from '$lib/components/kit/types';
import {
	orgConstitutionFor,
	candidateDetailFor,
	confidentialityFor,
	scopeOwnersFor,
	billingFor
} from '$lib/components/kit/fixtures';

// An org-zone section route. Resolves the org from the slug against real
// memberships (redirect to /you if not a member) and validates the section
// against the known org destinations (redirect to the org home for an unknown
// tail) — the two-nav never dead-ends.
//
// Tier-2 wiring: every org console section that has a Worker read-route now loads
// REAL `/v1` data via the SAME console clients the shipped `(console)` screens
// use, mapped to the kit shape by pure `*-map` mappers. Each fetch runs behind
// `guardTenantScope` on the resolved org's tenant key (`org.url`) and degrades to
// an empty kit list + a surfaced error (honest-empty / DJ1) on a live-service
// failure, a role-floor 403, or a dev-only 404 — so the screen always renders.
// Fetches are section-scoped: only the visited section's data is fetched.
//   • triage/approvals  → GET …/triage (maintainer floor)
//   • members/audit     → GET …/members · …/policies · …/audit (admin floor)
//   • identity          → GET …/identities (admin floor)
//   • incidents         → GET …/incidents (lead floor)
//   • clientaudit       → GET …/audit (admin floor — the tenant audit trail)
//   • health            → GET …/health (admin floor)
//   • engagements       → GET …/engagements (lead floor)
//   • billing           → GET …/billing (admin floor) — the LIVE billable seat
//     count overlaid on the illustrative pricing catalog (perSeat/tiers/invoices
//     stay fixture until a payment provider is wired, D-BILLING).
// The remaining sections (Overview ladder/projects, knowledge, confidentiality,
// scopes) still render off kit fixtures — their routes aren't built (Tier 3:
// scopes/projects/stance need further wiring).
export const load: PageLoad = async ({ parent, params, fetch }) => {
	const { memberships, accessToken, user } = await parent();
	const org = orgBySlug(memberships, params.slug);
	if (!org) redirect(307, '/you');
	if (!ORG_SECTIONS.includes(params.section)) redirect(307, `/org/${params.slug}`);
	const slug = params.slug;
	const section = params.section;
	const tenantKey = org.url;
	const opts = { fetch, accessToken };

	// A section-scoped guarded fetch: run only for `forSection`, degrade to
	// `empty` (+ surface the error message) on any failure so the screen renders.
	async function guardedFor<W, K>(
		forSection: string,
		empty: K,
		fetcher: (tk: string) => Promise<W>,
		map: (w: W) => K
	): Promise<{ value: K; error: string | null }> {
		if (section !== forSection) return { value: empty, error: null };
		try {
			const guarded = await guardTenantScope<W>(tenantKey, undefined as unknown as W, fetcher);
			// A membership-less guard returns the sentinel; treat as empty.
			if (guarded.noMembership) return { value: empty, error: null };
			return { value: map(guarded.value), error: null };
		} catch (e) {
			const error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
			return { value: empty, error };
		}
	}

	// Engagements (lead) — the shipped client register.
	const engagementsRes = await guardedFor<Engagement[], KitEngagement[]>(
		'engagements',
		[],
		(tk) => listEngagements(tk, opts),
		toKitEngagements
	);

	// Triage (maintainer) — the raw rows feed BOTH the triage groups and the
	// approvals list, so fetch once and derive both when either section is active.
	let triage: KitTriageGroup[] = [];
	let candidateDetail: KitCandidateDetail = candidateDetailFor(slug);
	let approvals: KitApproval[] = [];
	let triageError: string | null = null;
	if (section === 'triage' || section === 'approvals') {
		try {
			const guarded = await guardTenantScope<TriageRow[]>(tenantKey, [], (tk) => listTriage(tk, opts));
			const rows = guarded.value;
			triage = toKitTriageGroups(rows);
			approvals = toKitApprovals(rows);
			// The detail pane opens on the top-ranked candidate (the same first
			// selection ScrTriage seeds); an honest-empty detail when the queue is
			// clear.
			const top = flattenCandidates(triage)[0];
			const topRow = top ? rows.find((r) => r.signature === top.id) : undefined;
			candidateDetail = toKitCandidateDetail(topRow);
		} catch (e) {
			triageError = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
		}
	}

	// Members + Policies + Audit (admin) — the shared role-surfaces screen. Fetch
	// all three when either the members or the audit section is active.
	let members: KitMember[] = [];
	let rolePolicies: KitRolePolicy[] = toKitRolePolicies();
	let auditLog: KitChatTurn[] = [];
	let membersError: string | null = null;
	if (section === 'members' || section === 'audit') {
		try {
			const [m, p, a] = await Promise.all([
				guardTenantScope<Membership[]>(tenantKey, [], (tk) => listMembers(tk, opts)),
				guardTenantScope<Policy[]>(tenantKey, [], (tk) => listPolicies(tk, opts)),
				guardTenantScope<AuditEvent[]>(tenantKey, [], (tk) => listAudit(tk, opts))
			]);
			members = toKitMembers(m.value, { self: user?.id ?? undefined });
			rolePolicies = toKitRolePolicies(p.value);
			auditLog = toKitAuditThread(a.value);
		} catch (e) {
			membersError = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
		}
	}

	// Identity (admin).
	const identityRes = await guardedFor<Identity[], KitIdentity>(
		'identity',
		toKitIdentity([]),
		(tk) => listIdentities(tk, opts),
		toKitIdentity
	);

	// Incidents (lead).
	const incidentsRes = await guardedFor<{ incidents: Incident[]; open_count: number }, KitIncident[]>(
		'incidents',
		[],
		(tk) => listIncidents(tk, opts),
		(list) => toKitIncidents(list.incidents)
	);

	// Client audit (admin — the tenant audit trail projected onto the ledger).
	// Client-audit → the CONFIDENTIALITY ledger (audit_events filtered to
	// publish/contained/held, client-name enriched) — NOT the admin action-audit
	// (listAudit) the screen was wrongly bound to (client-audit.md resolved design).
	const clientAuditRes = await guardedFor<ClientAuditEntry[], KitClientAuditRow[]>(
		'clientaudit',
		[],
		(tk) => listClientAuditLedger(tk, opts),
		toKitClientAuditLedger
	);

	// Health (admin).
	const healthRes = await guardedFor<HealthRollup, KitHealth>(
		'health',
		toKitHealth({ connections: 0, queue_depth: 0, publish_rate_1h: 0, error_rate_1h: 0 }),
		(tk) => getHealth(tk, opts),
		toKitHealth
	);

	// Billing (admin) — the LIVE billable seat count (dojo.tenant_seat_usage) +
	// plan/renewal overlaid onto the illustrative pricing catalog (perSeat/tiers/
	// invoices stay fixture until a payment provider is wired, D-BILLING).
	const billingRes = await guardedFor<BillingResponse, KitBilling>(
		'billing',
		billingFor(slug),
		(tk) => getBilling(tk, opts),
		(res) => toKitBilling(billingFor(slug), res)
	);

	// Projects (org) — the tenant's dojo.projects (daemon-populated on relay runs).
	// Real read; honest-empty until populated — NOT the orgProjectsFor(slug) fixture.
	const projectsRes = await guardedFor<ProjectRow[], KitProject[]>(
		'projects',
		[],
		(tk) => listProjects(tk, opts),
		toKitProjects
	);

	// Knowledge (maintainer) — the tenant's published library over dojo.artifacts
	// (active / pending-prune / catalog). Real read; honest-empty on miss, error
	// state on failure — NOT the old knowledgeFor(slug) fixture (fabricated-data fix).
	const knowledgeRes = await guardedFor<KnowledgeLibraryWire, KitKnowledge>(
		'knowledge',
		{ prunePolicy: '', active: [], pending: [], catalog: [] },
		(tk) => getKnowledge(tk, opts),
		toKitKnowledge
	);

	return {
		slug,
		orgName: org.name,
		section,
		title: labelForSection(section, 'org'),
		// The tenant + token the screen's console mutations (set-role, incident +
		// engagement CRUD) call the /v1 write-routes with — passed through from the
		// resolved org + the layout so the actions POST/PATCH/DELETE directly.
		tenantKey,
		accessToken,
		// Overview-section data (fixtures — Tier 3, needs DDL).
		sections: orgConstitutionFor(slug),
		projects: projectsRes.value,
		projectsError: projectsRes.error,
		// Maintainer Govern consoles.
		triage,
		candidateDetail,
		approvals,
		triageError,
		knowledge: knowledgeRes.value,
		knowledgeError: knowledgeRes.error,
		// Lead Clients consoles.
		engagements: engagementsRes.value,
		engagementsError: engagementsRes.error,
		confidentiality: confidentialityFor(slug),
		incidents: incidentsRes.value,
		incidentsError: incidentsRes.error,
		clientAudit: clientAuditRes.value,
		clientAuditError: clientAuditRes.error,
		// Admin consoles. `roleTab` is the opening tab of the shared role-surfaces
		// screen — `members` → Members, `audit` → Audit.
		roleTab: tabForSection(section),
		members,
		rolePolicies,
		auditLog,
		membersError,
		scopeOwners: scopeOwnersFor(slug),
		identity: identityRes.value,
		identityError: identityRes.error,
		health: healthRes.value,
		healthError: healthRes.error,
		billing: billingRes.value,
		billingError: billingRes.error
	};
};
