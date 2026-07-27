<script lang="ts">
	import { onMount } from 'svelte';
	import { goto, invalidateAll } from '$app/navigation';
	import { SectionHead, Icon, EmptyState } from '$lib/components/kit';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import RelayStatusBadge from '$lib/components/RelayStatusBadge.svelte';
	import RelayGateCard from '$lib/components/RelayGateCard.svelte';
	import DojoJoinEmpty from '$lib/components/DojoJoinEmpty.svelte';
	import { requireTenant } from '$lib/org-guard';
	import { segmentStateBadge } from '$lib/relay-view';
	import { youHref } from '$lib/nav';
	import { subscribeRelay } from '$lib/relay-realtime';
	import { env } from '$env/dynamic/public';
	import type { RelaySegment } from '$lib/relay-data';

	// One personal run's detail — the drill-in behind /you/runs. Ports the shipped
	// (console) relay run VIEW into the dojo shell: a back header, the run identity
	// + live status, this run's pending "needs you" gate(s) answerable in place, and
	// the segment outline as a Phase → Step tree carrying each segment's state plus
	// its agent / model / spec-ref labels. Read-oriented: the console's PR-review
	// batch, offline queue, and nudge composer are the console power surface, out of
	// scope for this drill-in — gates are still answered here via the shared
	// RelayGateCard's built-in reply. The load (+page.ts) does the fetching + the
	// membership guard; this component is presentational plus the realtime refresh.
	// tenantKey/accessToken come from the (app) layout, merged onto `data`.
	let { data } = $props();

	function back() {
		goto(youHref('runs'));
	}

	// The run's goal, shown under the header (null-safe — `data.run` may be absent).
	const goal = $derived(data.run?.goal ?? null);

	// Phase → Step tree from the flat, seq-ordered segment list (mirrors the console
	// run view). Top-level segments (parent_id === null) are phases; the rest hang
	// under their parent, keeping the load's seq order within each group. An orphan
	// (a parent that isn't a top-level phase) surfaces at the top level so nothing is
	// silently dropped.
	interface PhaseNode {
		phase: RelaySegment;
		steps: RelaySegment[];
	}
	const outline = $derived.by<PhaseNode[]>(() => {
		const phases: PhaseNode[] = [];
		const byId: Record<string, PhaseNode> = {};
		for (const seg of data.segments) {
			if (seg.parent_id === null) {
				const node: PhaseNode = { phase: seg, steps: [] };
				phases.push(node);
				byId[seg.id] = node;
			}
		}
		for (const seg of data.segments) {
			if (seg.parent_id === null) continue;
			const parent = byId[seg.parent_id];
			if (parent) parent.steps.push(seg);
			else phases.push({ phase: seg, steps: [] }); // orphan — show standalone
		}
		return phases;
	});

	const done = $derived(data.segments.filter((s) => s.state === 'done').length);
	const total = $derived(data.segments.length);

	onMount(() =>
		// P4.2 realtime: subscribe to the signed-in user's relay rows so this run's
		// outline / inbox refresh LIVE (a raised gate, a segment or progress update).
		// Authorizes as the user (accessToken → session JWT) so RLS scopes it to their
		// own runs; a change debounces an invalidateAll. A no-op under SSR / when
		// unauthenticated. The returned teardown removes the channel on navigate-away.
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

{#snippet segMeta(seg: RelaySegment)}
	{#if seg.agent || seg.model || seg.spec_ref}
		<div
			class="mono text-ink-faint flex flex-wrap items-center gap-2 text-xs"
			style="margin-top: 6px"
		>
			{#if seg.agent}<span class="bg-paper-mute rounded" style="padding: 2px 6px">{seg.agent}</span>{/if}
			{#if seg.model}<span class="bg-paper-mute rounded" style="padding: 2px 6px">{seg.model}</span>{/if}
			{#if seg.spec_ref}<span>{seg.spec_ref}</span>{/if}
		</div>
	{/if}
{/snippet}

{#if data.noMembership}
	<div class="flex h-full w-full flex-col overflow-hidden">
		<DojoJoinEmpty what="relay runs" />
	</div>
{:else}
	<div class="flex flex-col p-8 gap-6">
		<button
			type="button"
			onclick={back}
			class="text-ink-mute inline-flex cursor-pointer items-center gap-1 self-start bg-transparent text-sm"
		>
			<Icon name="alt-arrow-left" size={15} toneClass="text-ink-mute" /> Back to runs
		</button>

		<SectionHead kanji="継" eyebrow="Relay · run" title={data.run?.title ?? 'Run'}>
			{#snippet right()}
				<div class="flex items-center gap-3">
					{#if data.run}
						<RelayStatusBadge status={data.run.status} />
					{/if}
					{#if total > 0}
						<span class="mono text-ink-mute text-xs">{done}/{total}</span>
					{/if}
				</div>
			{/snippet}
		</SectionHead>

		{#if goal}
			<p class="text-ink-soft text-sm" style="line-height: 1.55; margin: 0">{goal}</p>
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
			<!-- Needs you — this run's pending gate(s), answerable in place so the
				 away-from-keyboard loop doesn't dead-end at a run you can only watch. -->
			<div>
				<div class="mb-3 flex items-center gap-2">
					<span class="kanji text-accent text-sm">要</span>
					<span
						class="text-ink-mute text-xs font-semibold uppercase"
						style="letter-spacing: 0.14em">Needs you</span
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
		{/if}

		{#if outline.length > 0}
			<div class="flex flex-col gap-3">
				{#each outline as node (node.phase.id)}
					{@const phaseBadge = segmentStateBadge(node.phase.state)}
					<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-xl border">
						<!-- Phase header -->
						<div
							class="flex items-start gap-3 {node.steps.length > 0
								? 'border-paper-edge border-b'
								: ''}"
							style="padding: 16px"
						>
							<span
								class="mono text-ink-faint flex-shrink-0 text-xs"
								style="width: 22px; padding-top: 4px">{String(node.phase.seq).padStart(2, '0')}</span
							>
							<div class="flex-1" style="min-width: 0">
								<div class="flex flex-wrap items-center gap-2">
									<span class="text-ink text-sm font-semibold" style="flex: 1; min-width: 0"
										>{node.phase.title}</span
									>
									{#if node.phase.is_gate}
										<DojoChip
											toneClass={node.phase.gate_severity === 'blocking'
												? 'text-accent'
												: 'text-ink-mute'}
										>
											{node.phase.gate_severity === 'blocking' ? 'Gate · needs you' : 'Gate · heads-up'}
										</DojoChip>
									{/if}
									<DojoChip toneClass={phaseBadge.toneClass}>{phaseBadge.label}</DojoChip>
								</div>
								{#if node.phase.summary}
									<div class="text-ink-mute text-xs" style="margin-top: 4px; line-height: 1.5">
										{node.phase.summary}
									</div>
								{/if}
								{@render segMeta(node.phase)}
							</div>
						</div>

						<!-- Steps -->
						{#each node.steps as step, i (step.id)}
							{@const stepBadge = segmentStateBadge(step.state)}
							<div
								class={i < node.steps.length - 1 ? 'border-paper-edge border-b' : ''}
								style="padding: 12px 16px 12px 32px"
							>
								<div class="flex flex-wrap items-center gap-2">
									<span class="text-ink text-sm" style="flex: 1; min-width: 0">{step.title}</span>
									{#if step.is_gate}
										<DojoChip
											toneClass={step.gate_severity === 'blocking' ? 'text-accent' : 'text-ink-mute'}
										>
											{step.gate_severity === 'blocking' ? 'Gate · needs you' : 'Gate · heads-up'}
										</DojoChip>
									{/if}
									<DojoChip toneClass={stepBadge.toneClass}>{stepBadge.label}</DojoChip>
								</div>
								{#if step.summary}
									<div class="text-ink-mute text-xs" style="margin-top: 4px; line-height: 1.5">
										{step.summary}
									</div>
								{/if}
								{#if step.detail}
									<div class="text-ink-faint text-xs" style="margin-top: 4px; line-height: 1.5">
										{step.detail}
									</div>
								{/if}
								{@render segMeta(step)}
							</div>
						{/each}
					</div>
				{/each}
			</div>
		{/if}
	</div>
{/if}
