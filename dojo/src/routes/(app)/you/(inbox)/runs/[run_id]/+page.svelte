<script lang="ts">
	import { onMount } from 'svelte';
	import { goto, invalidateAll } from '$app/navigation';
	import {
		SectionHead,
		Icon,
		EmptyState,
		SubTabs,
		PlanOutline,
		PlanGraph,
		PlanBar,
		RunActivity
	} from '$lib/components/kit';
	import type { KitNavItem, KitTask } from '$lib/components/kit/types';
	import { planProgress } from '$lib/components/kit/plan';
	import RelayStatusBadge from '$lib/components/RelayStatusBadge.svelte';
	import RelayGateCard from '$lib/components/RelayGateCard.svelte';
	import DojoJoinEmpty from '$lib/components/DojoJoinEmpty.svelte';
	import { requireTenant } from '$lib/org-guard';
	import { segmentsToPlan, segmentsToActivity } from '$lib/relay-map';
	import { youHref } from '$lib/nav';
	import { subscribeRelay } from '$lib/relay-realtime';
	import { env } from '$env/dynamic/public';

	// One personal run's detail — the drill-in behind /you/runs. The run identity +
	// live status, this run's pending "needs you" gate(s) answerable in place, and
	// the run's plan rendered three ways from its REAL federated segments: an
	// Outline (phases → tasks), a Graph (stages flow), and the Activity timeline.
	// Read-oriented: the console's PR-review batch / offline queue / nudge composer
	// stay the console power surface. The load (+page.ts) does the fetching + the
	// membership guard; this component maps segments → kit shapes + realtime refresh.
	let { data } = $props();

	// <md the detail is a full pane pushed over the list — back returns to the inbox
	// list (/you). On md+ both panels show side by side, so the control is hidden.
	function back() {
		goto(youHref());
	}

	const goal = $derived(data.run?.goal ?? null);
	// The plan (phases → tasks) + the activity timeline, both from the real segments.
	const plan = $derived(segmentsToPlan(data.segments));
	const activity = $derived(segmentsToActivity(data.segments));
	const progress = $derived(planProgress(plan));
	const total = $derived(data.segments.length);

	// Which view of the plan — outline (default), graph, or the activity timeline.
	const VIEWS: KitNavItem[] = [
		{ id: 'outline', label: 'Outline', icon: 'layers-minimalistic' },
		{ id: 'graph', label: 'Graph', icon: 'diagram-up' },
		{ id: 'activity', label: 'Activity', icon: 'clock-circle' }
	];
	let view = $state('outline');
	let selectedId = $state<string | undefined>(undefined);
	const onSelect = (t: KitTask) => (selectedId = selectedId === t.id ? undefined : t.id);

	onMount(() =>
		// P4.2 realtime: subscribe to the signed-in user's relay rows so this run's
		// outline refreshes LIVE (a raised gate, a segment or progress update).
		subscribeRelay({
			url: env.PUBLIC_SUPABASE_URL,
			anonKey: env.PUBLIC_SUPABASE_ANON_KEY,
			accessToken: data.accessToken,
			topic: `relay:run:${data.runId}`,
			onChange: () => invalidateAll()
		})
	);
</script>

<svelte:head><title>{data.run?.title ?? 'Run'} · Dōjō</title></svelte:head>

{#if data.noMembership}
	<div class="flex h-full w-full flex-col overflow-hidden">
		<DojoJoinEmpty what="relay runs" />
	</div>
{:else}
	<div class="flex flex-col p-8 gap-6">
		<button
			type="button"
			onclick={back}
			class="text-ink-mute inline-flex cursor-pointer items-center gap-1 self-start bg-transparent text-sm md:hidden"
		>
			<Icon name="alt-arrow-left" size={15} toneClass="text-ink-mute" /> Back to inbox
		</button>

		<SectionHead kanji="継" eyebrow="Relay · run" title={data.run?.title ?? 'Run'}>
			{#snippet right()}
				{#if data.run}<RelayStatusBadge status={data.run.status} />{/if}
			{/snippet}
		</SectionHead>

		{#if goal}
			<p class="text-ink-soft text-sm" style="line-height: 1.55; margin: 0">{goal}</p>
		{/if}

		{#if total > 0}
			<div style="max-width: 560px">
				<div class="mb-2 flex items-center justify-between">
					<span class="mono text-ink-soft text-xs"
						>Phase {progress.stage} of {progress.stages}{progress.stageName
							? ` · ${progress.stageName}`
							: ''}</span
					>
					<span class="mono text-ink-soft text-xs"
						>{progress.done}/{progress.total} tasks · {progress.pct}%</span
					>
				</div>
				<PlanBar pct={progress.pct} tone="bg-ink" />
			</div>
		{/if}

		{#if data.error}
			<div
				class="bg-warning-soft border-warning-edge text-ink-soft flex items-center gap-2 rounded-xl border text-sm"
				style="padding: 12px 16px"
			>
				<span class="kanji text-warning">検</span>
				<span
					>Live relay is unavailable. <span class="mono text-ink-mute text-xs">{data.error}</span></span
				>
			</div>
		{/if}

		{#if data.gates.length > 0}
			<!-- Needs you — this run's pending gate(s), answerable in place. -->
			<div>
				<div class="mb-3 flex items-center gap-2">
					<span class="kanji text-accent text-sm">要</span>
					<span class="text-ink-mute text-xs font-semibold uppercase" style="letter-spacing: 0.14em"
						>Needs you</span
					>
				</div>
				<div class="flex flex-col gap-3">
					{#each data.gates as g (g.id)}
						<RelayGateCard
							gate={g}
							tenantKey={requireTenant(data.tenantKey)}
							accessToken={data.accessToken}
							onReplied={() => invalidateAll()}
							runHrefBase="/you/runs/"
						/>
					{/each}
				</div>
			</div>
		{/if}

		{#if total === 0 && !data.error}
			<EmptyState kanji="継" title="No outline yet.">
				The run hasn't published its plan. When it does, its phases and tasks appear here.
			</EmptyState>
		{:else if total > 0}
			<div class="flex flex-col gap-4">
				<SubTabs tabs={VIEWS} active={view} onPick={(id) => (view = id)} />
				{#if view === 'graph'}
					<PlanGraph {plan} {selectedId} {onSelect} />
				{:else if view === 'activity'}
					<RunActivity feed={activity} />
				{:else}
					<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-xl border">
						<PlanOutline {plan} {selectedId} {onSelect} />
					</div>
				{/if}
			</div>
		{/if}
	</div>
{/if}
