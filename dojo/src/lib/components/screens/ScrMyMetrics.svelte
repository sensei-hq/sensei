<script lang="ts">
	import { Screen } from '@sensei/ui';
	import { SectionHead } from '$lib/components/kit';
	import Spark from '$lib/components/kit/Spark.svelte';
	import type { MyMetricsView } from '$lib/my-metrics-view';
	// My metrics — a developer's own numbers, from what their daemon synced: the
	// Teams view's metrics, so a person's card reads like their lead's column.
	//
	// PRESENTATIONAL. The cards are shaped in my-metrics-view.ts; a failed read is
	// said, never shown as a quiet month.

	interface Props {
		/** Null when the read failed. */
		view: MyMetricsView | null;
		/** Non-null when the read FAILED: "nothing sent" and "could not ask" differ. */
		error?: string | null;
	}
	let { view, error = null }: Props = $props();
</script>

<Screen>
	{#snippet header()}
		<div class="px-4 pt-4 md:px-8 md:pt-8">
			<SectionHead
				eyebrow="You · metrics"
				title="My metrics"
				description="Your own numbers, from what sensei on your machine sent to your dōjōs."
			/>
		</div>
	{/snippet}
	<section class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
		{#if error || !view}
			<p role="alert" class="text-sm text-danger">Could not load your metrics — {error ?? 'no answer'}</p>
		{:else}
			<p class="text-xs text-ink-soft">The last {view.days} days, across the repositories you share.</p>
			<ul class="grid grid-cols-1 gap-3 md:grid-cols-2 md:gap-4">
				{#each view.cards as c (c.key)}
					<li class="flex flex-col gap-2 rounded border border-paper-edge p-4">
						<span class="text-xs text-ink-soft">{c.label}</span>
						{#if c.empty}
							<span class="text-sm text-ink-faint">Nothing sent in this window</span>
						{:else}
							<div class="flex items-end justify-between gap-3">
								<span class="text-2xl text-ink">{c.value}</span>
								{#if c.points.length > 1}
									<Spark data={c.points} />
								{/if}
							</div>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</section>
</Screen>
