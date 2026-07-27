<script lang="ts">
	import { onMount } from 'svelte';
	import { goto, invalidateAll } from '$app/navigation';
	import {
		KanjiToken,
		Icon,
		EmptyState,
		SubTabs,
		ListSection,
		PlanOutline,
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
	import { relativeAge } from '$lib/triage/view';
	import { youHref } from '$lib/nav';
	import { subscribeRelay } from '$lib/relay-realtime';
	import { env } from '$env/dynamic/public';

	// One personal run's detail (mockup RunDetail) — the right panel of the two-panel
	// inbox and the drill-in behind /you/runs. A session-identity header (short id +
	// live status + a REAL meta line), the run's pending "needs you" gate(s) answerable
	// in place, and the plan — all from the run's REAL federated segments. Two tabs
	// mirror the mockup: Needs you (the asks) · Plan (the outline); Activity +
	// Conversation sit below. The load (+page.ts) fetches + guards; this maps segments
	// → kit shapes + refreshes live over realtime.
	let { data } = $props();

	// <md the detail is a full pane pushed over the list — back returns to the inbox
	// list (/you). On md+ both panels show side by side, so the control is hidden.
	function back() {
		goto(youHref());
	}

	const goal = $derived(data.run?.goal ?? null);
	const plan = $derived(segmentsToPlan(data.segments));
	const activity = $derived(segmentsToActivity(data.segments));
	const progress = $derived(planProgress(plan));
	const total = $derived(data.segments.length);
	const needsYou = $derived(data.gates.length > 0);
	// The header glyph + progress bar go accent while something waits on you.
	const headTone = $derived(needsYou ? 'text-accent' : 'text-ink-mute');

	// A short session id + a meta line built ONLY from fields the zero-knowledge relay
	// feed actually carries (elapsed since start · last activity) — never a fabricated
	// project / model / edit count (F4 honesty).
	const sessionId = $derived((data.run?.run_id ?? data.runId ?? '').slice(0, 8));
	const metaBits = $derived(
		[
			data.run?.started_at ? relativeAge(data.run.started_at) : null,
			data.run?.last_event_at ? `last activity ${relativeAge(data.run.last_event_at)}` : null
		].filter((b): b is string => !!b)
	);

	// Two tabs (mockup): Needs you · Plan. Opens on the asks if any, else the plan;
	// resets only when the run changes — not on every realtime refresh.
	const TABS: KitNavItem[] = $derived([
		{
			id: 'needs',
			label: 'Needs you',
			icon: 'checklist-minimalistic',
			badge: data.gates.length || undefined
		},
		{ id: 'plan', label: 'Plan', icon: 'layers-minimalistic' }
	]);
	// Which tab is showing: the user's pick for THIS run if they chose one, else the
	// default — open on the asks if any, else the plan. A per-run pick map keeps this
	// pure ($derived, no reset effect): a new run falls back to its own default, and a
	// realtime refresh never clobbers the tab the user is on.
	let picks = $state<Record<string, string>>({});
	const tab = $derived(picks[data.runId] ?? (data.gates.length ? 'needs' : 'plan'));
	const pickTab = (id: string) => (picks = { ...picks, [data.runId]: id });

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
	<div class="flex flex-col gap-5 p-6">
		<button
			type="button"
			onclick={back}
			class="text-ink-mute inline-flex cursor-pointer items-center gap-1 self-start bg-transparent text-sm md:hidden"
		>
			<Icon name="alt-arrow-left" size={15} toneClass="text-ink-mute" /> Back to inbox
		</button>

		<!-- Header: session identity + a real meta line + live status (mockup RunDetail). -->
		<div class="flex items-baseline gap-3">
			<KanjiToken char="観" size="lg" toneClass={headTone} />
			<div style="min-width: 0; flex: 1">
				<div
					class="text-ink-mute text-xs font-semibold uppercase"
					style="letter-spacing: 0.18em"
				>
					Session · {sessionId}
				</div>
				<h2
					class="display text-ink text-xl font-normal"
					style="letter-spacing: -0.015em; margin: 2px 0 0; line-height: 1.1"
				>
					{data.run?.title ?? 'Run'}
				</h2>
				{#if metaBits.length}
					<div class="mono text-ink-mute text-xs" style="margin-top: 3px">
						{metaBits.join(' · ')}
					</div>
				{/if}
			</div>
			{#if data.run}<RelayStatusBadge status={data.run.status} />{/if}
		</div>

		{#if goal}
			<p class="text-ink-soft text-sm" style="line-height: 1.55; margin: 0; max-width: 560px">
				{goal}
			</p>
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
				<PlanBar pct={progress.pct} tone={needsYou ? 'bg-accent' : 'bg-ink'} />
			</div>
		{/if}

		{#if data.error}
			<div
				class="bg-warning-soft border-warning-edge text-ink-soft flex items-center gap-2 rounded-xl border text-sm"
				style="padding: 12px 16px"
			>
				<span class="kanji text-warning">検</span>
				<span
					>Live relay is unavailable.
					<span class="mono text-ink-mute text-xs">{data.error}</span></span
				>
			</div>
		{/if}

		<!-- Two tabs (mockup): Needs you · Plan. -->
		<div class="border-paper-edge border-b" style="padding-bottom: 12px">
			<SubTabs tabs={TABS} active={tab} onPick={pickTab} />
		</div>

		{#if tab === 'needs'}
			{#if data.gates.length > 0}
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
			{:else}
				<EmptyState kanji="静" title="Nothing waits on you.">
					sensei keeps going and surfaces only what it can't decide alone.
				</EmptyState>
			{/if}
		{:else if total > 0}
			<ListSection
				icon="layers-minimalistic"
				iconToneClass="text-ink-mute"
				title="Plan"
				count={`${progress.stages} phases · ${progress.total} tasks`}
			>
				{#if goal}
					<div
						class="border-paper-edge mono text-ink-mute border-b text-xs"
						style="padding: 12px 16px"
					>
						Goal · {goal}
					</div>
				{/if}
				<PlanOutline {plan} {selectedId} {onSelect} />
			</ListSection>
		{:else}
			<EmptyState kanji="継" title="No outline yet.">
				The run hasn't published its plan. When it does, its phases and tasks appear here.
			</EmptyState>
		{/if}

		<!-- Activity + Conversation (mockup RunDetail footer grid). -->
		<div class="grid grid-cols-1 items-start gap-4 md:grid-cols-2">
			<ListSection icon="history" iconToneClass="text-ink-mute" title="Activity">
				{#if activity.length > 0}
					<RunActivity feed={activity} />
				{:else}
					<div class="text-ink-mute text-sm" style="padding: 12px 16px">
						No activity yet — the run hasn't published events.
					</div>
				{/if}
			</ListSection>
			<ListSection icon="chat-round-line" iconToneClass="text-ink-mute" title="Conversation">
				<div class="text-ink-mute text-sm" style="padding: 12px 16px">
					Nothing said in this session yet.
				</div>
			</ListSection>
		</div>
	</div>
{/if}
