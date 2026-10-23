import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { ORG_SECTIONS, labelForSection } from '$lib/nav';
import { orgBySlug } from '$lib/chrome';
import { tabForSection } from '$lib/role-surfaces-view';
import { guardTenantScope } from '$lib/org-guard';
import {
	getKnowledge,
	listWorkspaces,
	DojoApiError,
	type KnowledgeLibraryWire,
	type WorkspaceRow
} from '$lib/client-data';
import { listTriage, type TriageRow } from '$lib/triage-data';
import { toViewerViews, type DojoViewerView, type DojoViewerWire, type ViewerWorkspaceView } from '$lib/viewers-view';
import { toTeamViews, type TeamView, type TeamWire } from '$lib/teams-view';
import { toTeamMetricsView, type TeamMetricsView } from '$lib/team-metrics-view';
import type { TeamMetricsWire } from '$lib/team-metrics';
import {
	listMembers,
	listPolicies,
	listIdentities,
	listAudit,
	listViewers,
	listTeams,
	getTeamMetrics,
	getHealth,
	getBilling,
	listTenantRules,
	type TenantRuleWire,
	type Membership,
	type Policy,
	type Identity,
	type AuditEvent,
	type HealthRollup,
	type BillingResponse
} from '$lib/admin-data';
import { toKitBilling, noBilling } from '$lib/billing-map';
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
import { toKitKnowledge } from '$lib/knowledge-map';
import { toKitWorkspaces } from '$lib/workspaces-map';
import type {
	KitTriageGroup,
	KitApproval,
	KitCandidateDetail,
	KitMember,
	KitRolePolicy,
	KitChatTurn,
	KitKnowledge,
	KitWorkspace,
	KitIdentity,
	KitHealth,
	KitBilling,
	KitConstitutionSection
} from '$lib/components/kit/types';
import { rulesToSections } from '$lib/constitution-map';

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
//   • health            → GET …/health (admin floor)
//   • billing           → GET …/billing (admin floor) — the LIVE billable seat
//     count, and the plan + renewal when an account exists; no price or invoice
//     exists before a payment provider is wired (D-BILLING), so none is shown.
//   • ladder            → GET …/rules (member) — the dōjō's published rules.
// Scopes has no source yet (no ownership is recorded), so it loads none and says
// so. No section reads the kit fixtures (#106).
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

	// Triage (maintainer) — the raw rows feed BOTH the triage groups and the
	// approvals list, so fetch once and derive both when either section is active.
	let triage: KitTriageGroup[] = [];
	let candidateDetail: KitCandidateDetail = toKitCandidateDetail(undefined);
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
	let viewers: DojoViewerView[] = [];
	let viewerWorkspaces: ViewerWorkspaceView[] = [];
	let teams: TeamView[] = [];
	let membersError: string | null = null;
	if (section === 'members' || section === 'audit') {
		try {
			const [m, p, a, v, pr, tm] = await Promise.all([
				guardTenantScope<Membership[]>(tenantKey, [], (tk) => listMembers(tk, opts)),
				guardTenantScope<Policy[]>(tenantKey, [], (tk) => listPolicies(tk, opts)),
				guardTenantScope<AuditEvent[]>(tenantKey, [], (tk) => listAudit(tk, opts)),
				// The Viewers tab (#307 D8): the dōjō's viewers and what they can be granted.
				guardTenantScope<DojoViewerWire[]>(tenantKey, [], (tk) => listViewers(tk, opts)),
				guardTenantScope<WorkspaceRow[]>(tenantKey, [], (tk) => listWorkspaces(tk, opts)),
				// The Teams tab (#307 D7).
				guardTenantScope<TeamWire[]>(tenantKey, [], (tk) => listTeams(tk, opts))
			]);
			teams = toTeamViews(tm.value);
			viewers = toViewerViews(v.value);
			viewerWorkspaces = pr.value.map((row) => ({ id: row.id, name: row.name }));
			members = toKitMembers(m.value, { self: user?.id ?? undefined });
			rolePolicies = toKitRolePolicies(p.value);
			auditLog = toKitAuditThread(a.value);
		} catch (e) {
			membersError = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
		}
	}

	// Teams (lead, #307 R8, D38) — the teams the caller is on, each member's
	// numbers. A failed read is an error state, never an empty team.
	const teamsRes = await guardedFor<TeamMetricsWire, TeamMetricsView | null>(
		'teams',
		null,
		(tk) => getTeamMetrics(tk, opts),
		toTeamMetricsView
	);

	// Identity (admin).
	const identityRes = await guardedFor<Identity[], KitIdentity>(
		'identity',
		toKitIdentity([]),
		(tk) => listIdentities(tk, opts),
		toKitIdentity
	);

	// Health (admin).
	const healthRes = await guardedFor<HealthRollup, KitHealth>(
		'health',
		toKitHealth({ connections: 0, queue_depth: 0, publish_rate_1h: 0, error_rate_1h: 0 }),
		(tk) => getHealth(tk, opts),
		toKitHealth
	);

	// Constitution (member) — the dōjō's published rules, grouped by scope.
	const ladderRes = await guardedFor<TenantRuleWire[], KitConstitutionSection[]>(
		'ladder',
		[],
		(tk) => listTenantRules(tk, opts),
		rulesToSections
	);

	// Billing (admin) — the LIVE billable seat count (dojo.tenant_seat_usage) and
	// the account's plan + renewal; nothing a payment provider would supply (#106).
	const billingRes = await guardedFor<BillingResponse, KitBilling>(
		'billing',
		noBilling(),
		(tk) => getBilling(tk, opts),
		toKitBilling
	);

	// Workspaces (org) — the tenant's dojo.workspaces (daemon-populated on relay runs).
	// Real read; honest-empty until populated — NOT the orgWorkspacesFor(slug) fixture.
	const workspacesRes = await guardedFor<WorkspaceRow[], KitWorkspace[]>(
		'workspaces',
		[],
		(tk) => listWorkspaces(tk, opts),
		toKitWorkspaces
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
		// The tenant + token the screen's console mutations (set-role, viewers,
		// teams, invites) call the /v1 write-routes with — passed through from the
		// resolved org + the layout so the actions POST/PATCH/DELETE directly.
		tenantKey,
		accessToken,
		// The dōjō's published rules, by scope (`…/rules`).
		sections: ladderRes.value,
		sectionsError: ladderRes.error,
		workspaces: workspacesRes.value,
		workspacesError: workspacesRes.error,
		// Maintainer Govern consoles.
		triage,
		candidateDetail,
		approvals,
		triageError,
		knowledge: knowledgeRes.value,
		knowledgeError: knowledgeRes.error,
		// Admin consoles. `roleTab` is the opening tab of the shared role-surfaces
		// screen — `members` → Members, `audit` → Audit.
		roleTab: tabForSection(section),
		members,
		rolePolicies,
		auditLog,
		viewers,
		viewerWorkspaces,
		teams,
		membersError,
		teamMetrics: teamsRes.value,
		teamMetricsError: teamsRes.error,
		// No scope ownership is recorded anywhere yet; the screen says so.
		scopeOwners: [],
		identity: identityRes.value,
		identityError: identityRes.error,
		health: healthRes.value,
		healthError: healthRes.error,
		billing: billingRes.value,
		billingError: billingRes.error
	};
};
