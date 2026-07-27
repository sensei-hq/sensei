<script lang="ts">
	import { goto, invalidateAll } from '$app/navigation';
	import ScrPlaceholder from '$lib/components/dojo2/ScrPlaceholder.svelte';
	import ScrProjects from '$lib/components/dojo2/ScrProjects.svelte';
	import ScrRelayWatch from '$lib/components/dojo2/ScrRelayWatch.svelte';
	import ScrRelayApprove from '$lib/components/dojo2/ScrRelayApprove.svelte';
	import ScrRelayDecide from '$lib/components/dojo2/ScrRelayDecide.svelte';
	import ScrRelayChat from '$lib/components/dojo2/ScrRelayChat.svelte';
	import ScrConstitution from '$lib/components/dojo2/ScrConstitution.svelte';
	import ScrRulePacks from '$lib/components/dojo2/ScrRulePacks.svelte';
	import ScrMyDojos from '$lib/components/dojo2/ScrMyDojos.svelte';
	import ScrContributions from '$lib/components/dojo2/ScrContributions.svelte';
	import { youHref } from '$lib/dojo2-nav';
	import { requireTenant } from '$lib/org-guard';
	import { replyToGate, sendNudge } from '$lib/relay-data';
	import type { KitProject, KitGate, KitDecision } from '$lib/components/kit/types';

	// The personal-zone section screen. Dispatches to the real screens for the
	// ported sections (projects · runs · approve · decide · chat · rules · packs ·
	// dojos · contributions). The Relay screens (runs · approve · decide · chat)
	// and My dōjōs now bind to real /v1 data the loader supplies; their actions
	// call the relay clients (replyToGate / sendNudge) against the layout tenant +
	// token and invalidateAll so the list refreshes. Opening a project routes to
	// the preview drill-in at /you/projects/[id]; Constitution's "Rule packs →"
	// link routes to /you/packs.
	let { data } = $props();

	function openProject(p: KitProject) {
		goto(youHref('projects') + '/' + p.id);
	}

	function goPacks() {
		goto(youHref('packs'));
	}

	// Answer a command gate (approve/deny) — a reply to the existing inbox row
	// (KitGate.id IS the relay_inbox id). Refresh the list on success so the
	// answered card falls away.
	async function replyGate(gate: KitGate, verdict: 'approve' | 'deny') {
		await replyToGate(requireTenant(data.tenantKey), gate.id, { verdict }, { accessToken: data.accessToken });
		await invalidateAll();
	}

	// Sign off a decision gate with the chosen option (KitDecision.id IS the
	// relay_inbox id).
	async function chooseDecision(decision: KitDecision, option: string) {
		await replyToGate(requireTenant(data.tenantKey), decision.id, { choice: option }, { accessToken: data.accessToken });
		await invalidateAll();
	}

	// Nudge the newest run TO the agent (a new inbox row, distinct from a gate
	// reply). No-op when there's no run to steer.
	async function sendChat(text: string) {
		if (!data.chatRunId) return;
		await sendNudge(requireTenant(data.tenantKey), data.chatRunId, text, { accessToken: data.accessToken });
		await invalidateAll();
	}
</script>

<svelte:head><title>{data.title} · Dōjō</title></svelte:head>

{#if data.section === 'projects'}
	<ScrProjects projects={data.projects} showDojo={false} onOpenProject={openProject} />
{:else if data.section === 'runs'}
	<ScrRelayWatch runs={data.runs} onOpenRun={(r) => goto(youHref('runs') + '/' + r.id)} />
{:else if data.section === 'approve'}
	<ScrRelayApprove
		gates={data.gates}
		onApprove={(g) => replyGate(g, 'approve')}
		onDeny={(g) => replyGate(g, 'deny')}
	/>
{:else if data.section === 'decide'}
	<ScrRelayDecide decisions={data.decisions} onChoose={chooseDecision} />
{:else if data.section === 'chat'}
	<ScrRelayChat thread={data.chat} me={data.me} onSend={sendChat} />
{:else if data.section === 'rules'}
	<ScrConstitution stance={data.stance} ladder={data.ladder} onGoPacks={goPacks} />
{:else if data.section === 'packs'}
	<ScrRulePacks packs={data.rulePacks} />
{:else if data.section === 'dojos'}
	<ScrMyDojos dojos={data.dojos} />
{:else if data.section === 'contributions'}
	<ScrContributions
		mine={data.contributionsMine}
		downstream={data.contributionsDownstream}
		stat={data.contributionsStat}
	/>
{:else}
	<ScrPlaceholder title={data.title} eyebrow="You" />
{/if}
