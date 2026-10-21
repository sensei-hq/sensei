<script lang="ts">
	import { goto, invalidateAll } from '$app/navigation';
	import { page } from '$app/state';
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
	import {
		setMemberRole,
		removeViewer,
		grantWorkspaceView,
		revokeWorkspaceView,
		createTeam,
		renameTeam,
		deleteTeam,
		addTeamMember,
		removeTeamMember,
		grantTeamWorkspaceView
	} from '$lib/admin-data';
	import {
		createIncident,
		updateIncident,
		deleteIncident,
		getIncident,
		updateEngagement,
		deleteEngagement,
		createEngagement,
		createInvite
	} from '$lib/client-data';
	import { OrgSectionState } from '$lib/org-section-state.svelte';
	import { InviteState } from '$lib/invite-state.svelte';
	import { TeamDraft } from '$lib/teams-state.svelte';
	import { toKitIncidentDetail } from '$lib/incidents-map';
	import type { KitProject } from '$lib/components/kit/types';

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

	function openProject(p: KitProject) {
		goto(orgHref(data.slug, 'projects') + '/' + p.id);
	}

	// The console writes, bound to the org's tenant and session token. Each reads
	// `data` when it runs, so a navigation to another org's section (the same
	// component, new params) writes to that org.
	const tk = () => requireTenant(data.tenantKey);
	const opts = () => ({ accessToken: data.accessToken });
	const org = new OrgSectionState(
		{
			setRole: (userId, role) => setMemberRole(tk(), userId, role, opts()),
			reportIncident: (title) => createIncident(tk(), { title }, opts()),
			resolveIncident: (id) => updateIncident(tk(), id, { resolved: true }, opts()),
			removeIncident: (id) => deleteIncident(tk(), id, opts()),
			openIncident: async (id) => toKitIncidentDetail(await getIncident(tk(), id, opts())),
			newEngagement: (clientName) => createEngagement(tk(), { client_name: clientName }, opts()),
			closeEngagement: (id) => updateEngagement(tk(), id, { status: 'ended' }, opts()),
			removeEngagement: (id) => deleteEngagement(tk(), id, opts()),
			removeViewer: (pid) => removeViewer(tk(), pid, opts()),
			grantView: (pid, projectId) => grantWorkspaceView(tk(), pid, projectId, opts()),
			revokeView: (pid, projectId) => revokeWorkspaceView(tk(), pid, projectId, opts()),
			saveTeam: (teamId, name) =>
				teamId ? renameTeam(tk(), teamId, name, opts()) : createTeam(tk(), name, opts()),
			deleteTeam: (teamId) => deleteTeam(tk(), teamId, opts()),
			addTeamMember: (teamId, pid) => addTeamMember(tk(), teamId, pid, opts()),
			removeTeamMember: (teamId, pid) => removeTeamMember(tk(), teamId, pid, opts()),
			grantTeamView: (teamId, projectId) => grantTeamWorkspaceView(tk(), teamId, projectId, opts())
		},
		invalidateAll
	);
	// The admin invite (#307 A3): member or viewer; the accept link is on this origin.
	const invite = new InviteState((input) => createInvite(tk(), input, opts()), page.url.origin);
	// New team / Rename (#307 A3).
	const teamDraft = new TeamDraft();
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
		onNew={org.newEngagement}
		onClose={org.closeEngagement}
		onDelete={org.removeEngagement}
	/>
{:else if data.section === 'incidents'}
	<ScrIncidents
		orgName={data.orgName}
		incidents={data.incidents}
		detail={org.incidentDetail}
		onOpen={(i) => org.openIncident(i)}
		onCloseDetail={() => org.closeIncident()}
		onReport={org.reportIncident}
		onResolve={org.resolveIncident}
		onDelete={org.removeIncident}
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
		onSetRole={(m, role) => org.setRole(m, role)}
		{invite}
		viewers={data.viewers}
		viewerProjects={data.viewerProjects}
		onGrantView={(v, projectId) => org.grantView(v, projectId)}
		onRevokeView={(v, projectId) => org.revokeView(v, projectId)}
		onRemoveViewer={(v) => org.removeViewer(v)}
		teams={data.teams}
		{teamDraft}
		onSaveTeam={(teamId, name) => org.saveTeam(teamId, name)}
		onDeleteTeam={(t) => org.deleteTeam(t)}
		onAddTeamMember={(t, pid) => org.addTeamMember(t, pid)}
		onRemoveTeamMember={(t, pid) => org.removeTeamMember(t, pid)}
		onGrantTeam={(t, projectId) => org.grantTeamView(t, projectId)}
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

{#if org.actionError}
	<!-- Honest surfacing of a failed console action — a dismissible toast, never a
	     silent no-op. Cleared on the next attempt. -->
	<div
		class="bg-danger-soft border-danger-soft text-ink fixed z-50 flex items-center gap-3 rounded-lg border text-sm"
		style="right: 16px; bottom: 16px; padding: 12px 16px; max-width: 380px"
		role="alert"
	>
		<span class="flex-1">{org.actionError}</span>
		<button
			type="button"
			class="text-ink-mute cursor-pointer text-xs underline"
			onclick={() => (org.actionError = null)}
		>
			dismiss
		</button>
	</div>
{/if}
