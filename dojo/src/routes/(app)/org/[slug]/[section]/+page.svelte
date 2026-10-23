<script lang="ts">
	import { goto, invalidateAll } from '$app/navigation';
	import { page } from '$app/state';
	import ScrOrgLadder from '$lib/components/screens/ScrOrgLadder.svelte';
	import ScrWorkspaces from '$lib/components/screens/ScrWorkspaces.svelte';
	import ScrTriage from '$lib/components/screens/ScrTriage.svelte';
	import ScrApprovals from '$lib/components/screens/ScrApprovals.svelte';
	import ScrKnowledge from '$lib/components/screens/ScrKnowledge.svelte';
	import ScrRoleSurfaces from '$lib/components/screens/ScrRoleSurfaces.svelte';
	import ScrScopes from '$lib/components/screens/ScrScopes.svelte';
	import ScrIdentity from '$lib/components/screens/ScrIdentity.svelte';
	import ScrIntegrations from '$lib/components/screens/ScrIntegrations.svelte';
	import ScrHealth from '$lib/components/screens/ScrHealth.svelte';
	import ScrTeams from '$lib/components/screens/ScrTeams.svelte';
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
		grantTeamWorkspaceView,
		listIntegrationUnits,
		claimIntegrationUnit,
		mapIntegrationUnit,
		listAzureConnections,
		startAzureConnect,
		syncAzure,
		removeAzureToken
	} from '$lib/admin-data';
	import { createInvite, importAzureOrganization, listAzureGrantOrganizations } from '$lib/client-data';
	import { OrgSectionState } from '$lib/org-section-state.svelte';
	import { InviteState } from '$lib/invite-state.svelte';
	import { TeamDraft } from '$lib/teams-state.svelte';
	import { IntegrationsState } from '$lib/integrations-state.svelte';
	import { AzureConnectState } from '$lib/azure-connect-state.svelte';
	import { connectGithub, type KavachSignIn } from '$lib/github-connect';
	import { providers } from '$kavach/providers';
	import { getContext, untrack } from 'svelte';
	import type { KitWorkspace } from '$lib/components/kit/types';

	// The org-zone section screen. Dispatches to the real screen for every NAV_ORG
	// destination off the kit fixtures the loader supplies: the Overview
	// (ladder · workspaces), the maintainer Govern consoles
	// (triage · approvals · knowledge) and the admin consoles
	// (members/scopes/identity/audit/health/billing) — the full dojo screen set,
	// so the trailing ScrPlaceholder branch is unreachable today and exists only
	// to catch a section added to NAV_ORG before its screen. The `members` and
	// `audit` sections share
	// ONE screen (ScrRoleSurfaces): the loader-supplied `roleTab` picks its opening
	// tab. Opening a workspace routes to the org workspace preview at
	// /org/[slug]/workspaces/[id] (an in-shell route so the URL stays the source of
	// truth — the shell keeps "Workspaces" active for the tail).
	let { data } = $props();

	// The signed-in viewer's real name (from the layout's loadConsoleContext) —
	// labels "you" in the audit surface. Never a fixture identity (F4).
	const viewer = $derived(data.user?.name ?? data.user?.email ?? 'You');

	function openWorkspace(p: KitWorkspace) {
		goto(orgHref(data.slug, 'workspaces') + '/' + p.id);
	}

	// The console writes, bound to the org's tenant and session token. Each reads
	// `data` when it runs, so a navigation to another org's section (the same
	// component, new params) writes to that org.
	const tk = () => requireTenant(data.tenantKey);
	const opts = () => ({ accessToken: data.accessToken });
	const org = new OrgSectionState(
		{
			setRole: (userId, role) => setMemberRole(tk(), userId, role, opts()),
			removeViewer: (pid) => removeViewer(tk(), pid, opts()),
			grantView: (pid, workspaceId) => grantWorkspaceView(tk(), pid, workspaceId, opts()),
			revokeView: (pid, workspaceId) => revokeWorkspaceView(tk(), pid, workspaceId, opts()),
			saveTeam: (teamId, name) =>
				teamId ? renameTeam(tk(), teamId, name, opts()) : createTeam(tk(), name, opts()),
			deleteTeam: (teamId) => deleteTeam(tk(), teamId, opts()),
			addTeamMember: (teamId, pid) => addTeamMember(tk(), teamId, pid, opts()),
			removeTeamMember: (teamId, pid) => removeTeamMember(tk(), teamId, pid, opts()),
			grantTeamView: (teamId, workspaceId) => grantTeamWorkspaceView(tk(), teamId, workspaceId, opts())
		},
		invalidateAll
	);
	// The admin invite (#307 A3): member or viewer; the accept link is on this origin.
	const invite = new InviteState((input) => createInvite(tk(), input, opts()), page.url.origin);
	// New team / Rename (#307 A3).
	const teamDraft = new TeamDraft();
	// Admin → Integrations (#308 R5a): GitHub, bound to this dōjō and session.
	// The GitHub token is the server's (a sealed cookie set when Connect GitHub
	// re-authorized through kavach); nothing here sends one.
	const kavach = getContext<KavachSignIn>('kavach');
	const integrations = new IntegrationsState({
		list: () => listIntegrationUnits(tk(), 'github', opts()),
		claim: (unitId) => claimIntegrationUnit(tk(), 'github', unitId, opts()),
		map: (unitId, target) => mapIntegrationUnit(tk(), 'github', unitId, target, opts()),
		connect: () => connectGithub(kavach, providers, page.url.href)
	});
	// Azure DevOps (#308 R5b, D42): connect through Microsoft Entra, import the
	// organizations into this dōjō; then this dōjō's Azure organizations, each
	// with its state, Sync and Remove. No token passes through here.
	const azure = new AzureConnectState({
		start: () => startAzureConnect(tk(), opts()),
		navigate: (url) => window.location.assign(url),
		organizations: () => listAzureGrantOrganizations(opts()),
		importOrg: (organization) => importAzureOrganization(organization, opts()),
		status: () => listAzureConnections(tk(), opts()),
		sync: (organizationId) => syncAzure(tk(), organizationId, opts()),
		removeToken: (organizationId) => removeAzureToken(tk(), organizationId, opts())
	});
	// Read on opening the section, again for another org (this component is
	// reused), and again after the GitHub connection returns (kavach's sync
	// invalidates the data, which re-runs this).
	$effect(() => {
		const current = data; // a new object after every load, the reload included
		if (current.section !== 'integrations') return;
		untrack(() => {
			integrations.reset();
			void integrations.load();
			azure.reset();
			azure.notice(page.url.searchParams);
			void azure.load();
		});
	});
</script>

<svelte:head><title>{data.title} · {data.orgName} · Dōjō</title></svelte:head>

{#if data.section === 'ladder'}
	<ScrOrgLadder orgName={data.orgName} sections={data.sections} />
{:else if data.section === 'workspaces'}
	<ScrWorkspaces
		workspaces={data.workspaces}
		showDojo={false}
		eyebrow={data.orgName + ' · jurisdiction'}
		title="Workspaces"
		onOpenWorkspace={openWorkspace}
	/>
{:else if data.section === 'triage'}
	<ScrTriage orgName={data.orgName} groups={data.triage} detail={data.candidateDetail} />
{:else if data.section === 'approvals'}
	<ScrApprovals orgName={data.orgName} approvals={data.approvals} />
{:else if data.section === 'knowledge'}
	<ScrKnowledge orgName={data.orgName} knowledge={data.knowledge} />
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
		viewerWorkspaces={data.viewerWorkspaces}
		onGrantView={(v, workspaceId) => org.grantView(v, workspaceId)}
		onRevokeView={(v, workspaceId) => org.revokeView(v, workspaceId)}
		onRemoveViewer={(v) => org.removeViewer(v)}
		teams={data.teams}
		{teamDraft}
		onSaveTeam={(teamId, name) => org.saveTeam(teamId, name)}
		onDeleteTeam={(t) => org.deleteTeam(t)}
		onAddTeamMember={(t, pid) => org.addTeamMember(t, pid)}
		onRemoveTeamMember={(t, pid) => org.removeTeamMember(t, pid)}
		onGrantTeam={(t, workspaceId) => org.grantTeamView(t, workspaceId)}
	/>
{:else if data.section === 'teams'}
	<ScrTeams orgName={data.orgName} view={data.teamMetrics} error={data.teamMetricsError} />
{:else if data.section === 'scopes'}
	<ScrScopes orgName={data.orgName} owners={data.scopeOwners} />
{:else if data.section === 'identity'}
	<ScrIdentity orgName={data.orgName} identity={data.identity} />
{:else if data.section === 'integrations'}
	<ScrIntegrations orgName={data.orgName} state={integrations} {azure} />
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
