<script lang="ts">
	import { Notice } from '@sensei/ui';
	import RepoMetricCards from '$lib/components/RepoMetricCards.svelte';
	import { relativeAge } from '$lib/admin-view';

	// One repository's screen (#532): identity, recency, numbers, diagrams.
	// The header names the repo (name, key, visibility, activity, tracked
	// branches); the metric cards are the shared per-repo cards; the diagrams
	// link preserves the branch. A failed metrics read is an error state,
	// never empty charts.
	let { data } = $props();

	const diagramsHref = $derived(
		data.branch
			? `/org/${data.slug}/repositories/${data.repo.id}/diagrams/structure?branch=${encodeURIComponent(data.branch)}`
			: `/org/${data.slug}/repositories/${data.repo.id}/diagrams/structure`
	);
</script>

<svelte:head><title>{data.repo.name} · {data.orgName} · Dōjō</title></svelte:head>

<div class="flex flex-col gap-4 p-4 md:p-8" data-testid="repo-screen">
	<div>
		<h1 class="m-0 text-lg">{data.repo.name}</h1>
		<p class="m-0 font-mono text-xs text-ink-mute">{data.repo.key}</p>
		<p class="m-0 text-xs text-ink-mute">
			{data.repo.visibility ?? 'unknown visibility'}
			{#if data.repo.last_activity_at}
				· Active {relativeAge(data.repo.last_activity_at)}
			{:else}
				· No activity yet
			{/if}
			· {data.repo.branches.length === 0
				? 'no tracked branch'
				: `branches: ${data.repo.branches.map((b) => b.branch).join(', ')}`}
		</p>
	</div>

	<nav>
		<a href={diagramsHref} data-testid="repo-diagrams-link">Diagrams</a>
	</nav>

	{#if data.metricsError}
		<Notice size="panel" title="Could not load this repository’s metrics" detail={data.metricsError} />
	{:else if data.metricsView && data.metricsView.repos.length === 0}
		<p class="text-sm text-ink-faint">Nothing sent in this window.</p>
	{:else if data.metricsView}
		<RepoMetricCards repos={data.metricsView.repos} />
	{/if}
</div>
