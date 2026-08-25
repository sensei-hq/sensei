<script lang="ts">
	import { goto, invalidateAll } from '$app/navigation';
	import ScrOrgLadder from '$lib/components/screens/ScrOrgLadder.svelte';
	import ScrProjects from '$lib/components/screens/ScrProjects.svelte';
	import ScrTriage from '$lib/components/screens/ScrTriage.svelte';
	import ScrApprovals from '$lib/components/screens/ScrApprovals.svelte';
	import ScrKnowledge from '$lib/components/screens/ScrKnowledge.svelte';
	import ScrEngagements from '$lib/components/screens/ScrEngagements.svelte';
	import ScrIncidents from '$lib/components/screens/ScrIncidents.svelte';
	import ScrClientAudit from '$lib/components/screens/ScrClientAudit.svelte';
	import ScrRoleSurfaces from '$lib/components/screens/ScrRoleSurfaces.svelte';
	import ScrScopes from '$lib/components/screens/ScrScopes.svelte';
	import ScrIdentity from '$lib/components/screens/ScrIdentity.svelte';
	import ScrHealth from '$lib/components/screens/ScrHealth.svelte';
	import ScrBilling from '$lib/components/screens/ScrBilling.svelte';
	import ScrPlaceholder from '$lib/components/screens/ScrPlaceholder.svelte';
	import { orgHref, labelForSection } from '$lib/nav';
	import { requireTenant } from '$lib/org-guard';
	import { setMemberRole } from '$lib/admin-data';
	import {
		createIncident,
		updateIncident,
		deleteIncident,
		getIncident,
		updateEngagement,
		deleteEngagement,
		createEngagement,
		DojoApiError
	} from '$lib/client-data';
	import { toKitIncidentDetail } from '$lib/incidents-map';
	import type {
		KitProject,
		KitMember,
		KitIncident,
		KitIncidentDetail,
		KitEngagement
	} from '$lib/components/kit/types';

	// The org-zone section screen. Dispatches to the real screen for every NAV_ORG
	// destination off the kit fixtures the loader supplies: the Overview
	// (ladder · projects), the maintainer Govern consoles
	// (triage · approvals · knowledge), the lead Clients consoles
	// (engagements · incidents · clientaudit) and the admin consoles
	// (members/scopes/identity/audit/health/billing) — the full dojo screen set,
	// so the trailing ScrPlaceholder branch is unreachable today and exists only
	// to catch a section added to NAV_ORG before its screen. The `members` and
	// `audit` sections share
	// ONE screen (ScrRoleSurfaces): the loader-supplied `roleTab` picks its opening
	// tab. Opening a project routes to the org project preview at
	// /org/[slug]/projects/[id] (an in-shell route so the URL stays the source of
	// truth — the shell keeps "Projects" active for the tail).
	let { data } = $props();

	// The signed-in viewer's real name (from the layout's loadConsoleContext) —
	// labels "you" in the audit surface. Never a fixture identity (F4).
	const viewer = $derived(data.user?.name ?? data.user?.email ?? 'You');

	// A live-action error surfaced honestly under the screen (no silent failure).
	// Cleared before each attempt; set from the API `error` message on a non-2xx.
	let actionError = $state<string | null>(null);

	function openProject(p: KitProject) {
		goto(orgHref(data.slug, 'projects') + '/' + p.id);
	}

	// Run a console mutation against the resolved org tenant + session token, then
	// reload so the list reflects the write (invalidateAll re-runs the loader).
	// A failure surfaces the API message rather than failing silently.
	async function act(fn: () => Promise<unknown>) {
		actionError = null;
		try {
			await fn();
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'that action could not be completed';
		}
	}

	const tk = () => requireTenant(data.tenantKey);
	const opts = () => ({ accessToken: data.accessToken });

	// Admin — set a member's dōjō role (the row carries the underlying user_id).
	function setRole(m: KitMember, role: string) {
		if (!m.userId) return;
		void act(() => setMemberRole(tk(), m.userId as string, role, opts()));
	}

	// Lead — incident report / resolve / delete.
	const reportIncident = (title: string) => act(() => createIncident(tk(), { title }, opts()));
	const resolveIncident = (i: KitIncident) => act(() => updateIncident(tk(), i.id, { resolved: true }, opts()));
	const removeIncident = (i: KitIncident) => act(() => deleteIncident(tk(), i.id, opts()));

	// Lead — open an incident's detail pane (GET …/incidents/{id} on demand).
	// A failed fetch surfaces honestly rather than opening an empty pane.
	let incidentDetail = $state<KitIncidentDetail | null>(null);
	async function openIncident(i: KitIncident) {
		actionError = null;
		try {
			incidentDetail = toKitIncidentDetail(await getIncident(tk(), i.id, opts()));
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not open that incident';
		}
	}
	const closeIncidentDetail = () => (incidentDetail = null);

	// Lead — engagement new / close / delete.
	const newEngagement = (clientName: string) => act(() => createEngagement(tk(), { client_name: clientName }, opts()));
	const closeEngagement = (e: KitEngagement) => act(() => updateEngagement(tk(), e.id, { status: 'ended' }, opts()));
	const removeEngagement = (e: KitEngagement) => act(() => deleteEngagement(tk(), e.id, opts()));
</script>

<svelte:head><title>{data.title} · {data.orgName} · Dōjō</title></svelte:head>

{#if data.section === 'ladder'}
	<ScrOrgLadder orgName={data.orgName} sections={data.sections} />
{:else if data.section === 'projects'}
	<ScrProjects
		projects={data.projects}
		showDojo={false}
		eyebrow={data.orgName + ' · jurisdiction'}
		title="Projects"
		onOpenProject={openProject}
	/>
{:else if data.section === 'triage'}
	<ScrTriage orgName={data.orgName} groups={data.triage} detail={data.candidateDetail} />
{:else if data.section === 'approvals'}
	<ScrApprovals orgName={data.orgName} approvals={data.approvals} />
{:else if data.section === 'knowledge'}
	<ScrKnowledge orgName={data.orgName} knowledge={data.knowledge} />
{:else if data.section === 'engagements'}
	<ScrEngagements
		orgName={data.orgName}
		engagements={data.engagements}
		confidentiality={data.confidentiality}
		onNew={newEngagement}
		onClose={closeEngagement}
		onDelete={removeEngagement}
	/>
{:else if data.section === 'incidents'}
	<ScrIncidents
		orgName={data.orgName}
		incidents={data.incidents}
		detail={incidentDetail}
		onOpen={openIncident}
		onCloseDetail={closeIncidentDetail}
		onReport={reportIncident}
		onResolve={resolveIncident}
		onDelete={removeIncident}
	/>
{:else if data.section === 'clientaudit'}
	<ScrClientAudit orgName={data.orgName} entries={data.clientAudit} />
{:else if data.section === 'members' || data.section === 'audit'}
	<ScrRoleSurfaces
		orgName={data.orgName}
		tab={data.roleTab}
		members={data.members}
		policies={data.rolePolicies}
		audit={data.auditLog}
		me={viewer}
		onSetRole={setRole}
	/>
{:else if data.section === 'scopes'}
	<ScrScopes orgName={data.orgName} owners={data.scopeOwners} />
{:else if data.section === 'identity'}
	<ScrIdentity orgName={data.orgName} identity={data.identity} />
{:else if data.section === 'health'}
	<ScrHealth orgName={data.orgName} health={data.health} />
{:else if data.section === 'billing'}
	<ScrBilling orgName={data.orgName} billing={data.billing} />
{:else}
	<!-- Named branches only. This used to be an unlabelled `{:else}` holding
	     billing, which happened to be correct because ORG_SECTIONS covered
	     exactly the branches above — but it meant a newly-added section would
	     silently render Plan & billing, and the house rule is explicit that
	     money-facing screens don't get reached by a fallthrough. The loader
	     already redirects sections outside ORG_SECTIONS, so this is the
	     in-nav-but-not-yet-built case. -->
	<ScrPlaceholder title={labelForSection(data.section, 'org')} eyebrow={data.orgName} />
{/if}

{#if actionError}
	<!-- Honest surfacing of a failed console action — a dismissible toast, never a
	     silent no-op. Cleared on the next attempt. -->
	<div
		class="bg-danger-soft border-danger-soft text-ink fixed z-50 flex items-center gap-3 rounded-lg border text-sm"
		style="right: 16px; bottom: 16px; padding: 12px 16px; max-width: 380px"
		role="alert"
	>
		<span class="flex-1">{actionError}</span>
		<button
			type="button"
			class="text-ink-mute cursor-pointer text-xs underline"
			onclick={() => (actionError = null)}
		>
			dismiss
		</button>
	</div>
{/if}
