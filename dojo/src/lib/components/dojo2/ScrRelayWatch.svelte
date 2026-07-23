<script lang="ts">
	import { SectionHead, RunCard, EmptyState } from '$lib/components/kit';
	import type { KitRun } from '$lib/components/kit/types';

	// Relay · watch (mockup ScrRelayWatch) — the live-runs list. A flush card of
	// RunCards. Presentational: the page supplies the runs (kit fixtures this
	// chunk). `onOpenRun` bubbles a row open; `mobile` switches to stacked rows.
	// Degrades to an honest empty state when nothing is running (DJ1).
	let {
		runs = [],
		mobile = false,
		onOpenRun
	}: {
		runs?: KitRun[];
		mobile?: boolean;
		onOpenRun?: (run: KitRun) => void;
	} = $props();
</script>

<div class="flex flex-col {mobile ? 'p-4 gap-4' : 'p-8 gap-6'}">
	<SectionHead eyebrow="Relay · watch" title="Live runs" count={runs.length}>
		{#snippet right()}
			<a href="#session-log" class="mono text-ink-mute text-xs no-underline">session log →</a>
		{/snippet}
	</SectionHead>

	{#if runs.length}
		<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
			{#each runs as run (run.id)}
				<RunCard {run} flat stacked={mobile} onOpen={onOpenRun} />
			{/each}
		</div>
	{:else}
		<EmptyState kanji="静" title="No sessions running.">
			Live sessions across your dōjōs appear here as they run. When it is quiet, sensei stays quiet.
		</EmptyState>
	{/if}
</div>
