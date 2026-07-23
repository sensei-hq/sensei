<script lang="ts">
	import { goto } from '$app/navigation';
	import ScrOrgLadder from '$lib/components/dojo2/ScrOrgLadder.svelte';
	import ScrProjects from '$lib/components/dojo2/ScrProjects.svelte';
	import ScrTriage from '$lib/components/dojo2/ScrTriage.svelte';
	import ScrApprovals from '$lib/components/dojo2/ScrApprovals.svelte';
	import ScrKnowledge from '$lib/components/dojo2/ScrKnowledge.svelte';
	import ScrEngagements from '$lib/components/dojo2/ScrEngagements.svelte';
	import ScrIncidents from '$lib/components/dojo2/ScrIncidents.svelte';
	import ScrClientAudit from '$lib/components/dojo2/ScrClientAudit.svelte';
	import ScrRoleSurfaces from '$lib/components/dojo2/ScrRoleSurfaces.svelte';
	import ScrScopes from '$lib/components/dojo2/ScrScopes.svelte';
	import ScrIdentity from '$lib/components/dojo2/ScrIdentity.svelte';
	import ScrHealth from '$lib/components/dojo2/ScrHealth.svelte';
	import ScrBilling from '$lib/components/dojo2/ScrBilling.svelte';
	import { orgHref } from '$lib/dojo2-nav';
	import { me } from '$lib/components/kit/fixtures';
	import type { KitProject } from '$lib/components/kit/types';

	// The org-zone section screen. Dispatches to the real screen for every NAV_ORG
	// destination off the kit fixtures the loader supplies: the Overview
	// (ladder · projects), the maintainer Govern consoles
	// (triage · approvals · knowledge), the lead Clients consoles
	// (engagements · incidents · clientaudit) and the admin consoles
	// (members/scopes/identity/audit/health/billing) — the full dojo2 screen set,
	// no placeholder destination left. The `members` and `audit` sections share
	// ONE screen (ScrRoleSurfaces): the loader-supplied `roleTab` picks its opening
	// tab. Opening a project routes to the org project preview at
	// /org/[slug]/projects/[id] (an in-shell route so the URL stays the source of
	// truth — the shell keeps "Projects" active for the tail).
	let { data } = $props();

	function openProject(p: KitProject) {
		goto(orgHref(data.slug, 'projects') + '/' + p.id);
	}
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
	/>
{:else if data.section === 'incidents'}
	<ScrIncidents orgName={data.orgName} incidents={data.incidents} />
{:else if data.section === 'clientaudit'}
	<ScrClientAudit orgName={data.orgName} entries={data.clientAudit} />
{:else if data.section === 'members' || data.section === 'audit'}
	<ScrRoleSurfaces
		orgName={data.orgName}
		tab={data.roleTab}
		members={data.members}
		policies={data.rolePolicies}
		audit={data.auditLog}
		me={me.name}
	/>
{:else if data.section === 'scopes'}
	<ScrScopes orgName={data.orgName} owners={data.scopeOwners} />
{:else if data.section === 'identity'}
	<ScrIdentity orgName={data.orgName} identity={data.identity} />
{:else if data.section === 'health'}
	<ScrHealth orgName={data.orgName} health={data.health} />
{:else}
	<ScrBilling orgName={data.orgName} billing={data.billing} />
{/if}
