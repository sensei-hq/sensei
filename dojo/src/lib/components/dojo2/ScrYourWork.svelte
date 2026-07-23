<script lang="ts">
	import {
		SectionHead,
		Banner,
		StatBadge,
		NeedsYouBand,
		ListSection,
		RunCard,
		ProjectRow,
		Btn,
		EmptyState
	} from '$lib/components/kit';
	import type { KitNeed, KitRun, KitProject } from '$lib/components/kit/types';
	import type { NeedsAction } from '$lib/components/kit/needs';
	import { untrack } from 'svelte';
	import { createYourWork, primaryLabel, actForKind } from '$lib/your-work-view.svelte';

	// The Your-work landing body (mockup ScrYourWork) — the needs-you band, week
	// stat badges, live runs, and active projects. Presentational: it takes the
	// data as props (the page supplies kit fixtures this chunk) and owns only the
	// resolve-inline state via `createYourWork`. `onOpenProject` bubbles a project
	// open; `mobile` switches to the stacked phone layout (no stat row).
	let {
		needsYou = [],
		runs = [],
		projects = [],
		mobile = false,
		onOpenProject
	}: {
		needsYou?: KitNeed[];
		runs?: KitRun[];
		projects?: KitProject[];
		mobile?: boolean;
		onOpenProject?: (p: KitProject) => void;
	} = $props();

	// Seed the resolve-inline state ONCE from the page-load props (they don't
	// change during a mount — a navigation re-mounts the landing), so the
	// `resolved` map persists as the viewer acts. `untrack` makes that initial
	// capture explicit rather than a reactivity smell.
	const work = untrack(() => createYourWork(needsYou, { projects, runs }));

	// The needs-you band acts inline — map its action back to the resolve reducer.
	function onAct(item: KitNeed, action: NeedsAction) {
		work.act(item, action.id);
	}

	// The "start here" banner leads with the most-urgent open need's primary act.
	function startHere() {
		if (work.next) work.act(work.next, actForKind(work.next.kind));
	}
</script>

<div class="flex flex-col" style="padding: {mobile ? '16px' : '32px'}; gap: {mobile ? '16px' : '24px'}">
	<SectionHead eyebrow="Monday · morning" title="Your work" />

	{#if work.next}
		{@const n = work.next}
		<Banner kanji="即" tone="accent" title={'Start here — ' + n.title}>
			{n.project} · {n.dojo} · {n.why}. {work.openItems.length - 1 > 0
				? work.openItems.length - 1 + ' more below.'
				: "Then you're clear."}
			{#snippet right()}
				<Btn size="sm" icon="check-circle" onclick={startHere}>{primaryLabel(n.kind)}</Btn>
			{/snippet}
		</Banner>
	{:else if needsYou.length}
		<Banner kanji="静" tone="success" title="Nothing needs you.">
			Sessions are running within the rules you set.
		</Banner>
	{/if}

	{#if !mobile && (runs.length || projects.length || needsYou.length)}
		<div
			class="bg-paper-soft border-paper-edge flex items-center rounded-lg border"
			style="padding: 16px 32px; gap: 48px"
		>
			<StatBadge n={work.runsWeek} label="runs this week" sub="↑ from 9" toneClass="text-accent" />
			<span class="bg-paper-edge" style="width: 1px; height: 34px; flex-shrink: 0"></span>
			<StatBadge n={work.openItems.length} label="need you" sub="gates · conflicts · reviews" />
			<span class="bg-paper-edge" style="width: 1px; height: 34px; flex-shrink: 0"></span>
			<StatBadge n="0" label="incidents" sub="30 days clean" toneClass="text-success" />
		</div>
	{/if}

	{#if needsYou.length}
		<NeedsYouBand items={work.ordered} resolved={work.resolved} {onAct} {mobile} />
	{/if}

	{#if runs.length}
		<ListSection
			icon="eye"
			iconToneClass="text-success"
			title="Live runs"
			count={work.activeRuns + ' running'}
			countToneClass="text-success"
		>
			{#each runs as run (run.id)}
				<RunCard {run} flat stacked={mobile} />
			{/each}
		</ListSection>
	{/if}

	{#if projects.length}
		<ListSection icon="folder" title="Active projects" count={projects.length}>
			{#snippet right()}
				<Btn size="sm" variant="ghost" icon="tuning-2">Filter</Btn>
			{/snippet}
			{#each projects as p (p.id)}
				<ProjectRow {p} onopen={onOpenProject} showDojo={!mobile} compact={mobile} />
			{/each}
		</ListSection>
	{/if}

	{#if !needsYou.length && !runs.length && !projects.length}
		<EmptyState kanji="空" title="Nothing in flight yet.">
			Your work — live runs, the needs-you band, and projects in flight — appears here once you
			join a dōjō or sensei starts watching a session on this machine.
		</EmptyState>
	{/if}
</div>
