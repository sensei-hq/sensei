<script lang="ts">
	import Spark from '$lib/components/kit/Spark.svelte';
	import { relativeAge } from '$lib/admin-view';
	import type { WorkspaceRepoMetrics } from '$lib/workspace-metrics-view';

	// Per-repository metric cards, shared by the workspace drill-in and the
	// repository screen (#532): one block per repo in the order given (callers
	// sort most-recently-active first), each card with its sparkline. The
	// activity shown is the sort key the view sorted by.
	let { repos }: { repos: WorkspaceRepoMetrics[] } = $props();
</script>

<div class="flex flex-col gap-5">
	{#each repos as repo (repo.repository_id)}
		<div>
			<div class="mono text-ink-soft text-xs mb-2">
				{repo.name}
				{#if repo.last_activity_at}
					<span class="text-ink-faint"> · Active {relativeAge(repo.last_activity_at)}</span>
				{/if}
			</div>
			<ul class="grid grid-cols-1 gap-3 md:grid-cols-2 md:gap-4">
				{#each repo.cards as c (c.key)}
					<li class="flex flex-col gap-2 rounded border border-paper-edge p-4">
						<span class="text-xs text-ink-mute">{c.label}</span>
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
		</div>
	{/each}
</div>
