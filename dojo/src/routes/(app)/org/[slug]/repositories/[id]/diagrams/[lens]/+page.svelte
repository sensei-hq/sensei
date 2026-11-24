<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { untrack } from 'svelte';
	import { CodeMap, CodeMapState, type Lens } from '@sensei/codemap';
	import { graphPortClient } from '$lib/graph-port-client';

	// One repository's diagrams (#268): the shared Code map over the dōjō's
	// graph reads, with the branch picker and the lens switcher. The map's
	// state is keyed by repository + branch, so switching branch re-reads
	// everything instead of showing the old branch's picture. Untracked lens
	// switching, as the app's page: drilling in must not climb back out.
	let { data } = $props();

	const LENSES: { id: Lens; label: string }[] = [
		{ id: 'structure', label: 'Structure' },
		{ id: 'wheel', label: 'Wheel' },
		{ id: 'zones', label: 'Zones' },
		{ id: 'matrix', label: 'Matrix' },
		{ id: 'cycles', label: 'Cycles' },
		{ id: 'coupling', label: 'Coupling' },
		{ id: 'complexity', label: 'Complexity' }
	];

	const mapKey = $derived(`${data.repo.id}:${data.branch}`);
	let current: { key: string; state: CodeMapState } | null = null;
	const state = $derived.by(() => {
		if (current?.key !== mapKey) current = { key: mapKey, state: new CodeMapState() };
		return (current as { state: CodeMapState }).state;
	});
	const port = $derived(
		graphPortClient(() => ({ repoKey: data.repo.key, branch: data.branch ?? '', fetch, accessToken: data.accessToken }))
	);

	$effect(() => {
		const [map, lens] = [state, data.lens as Lens];
		untrack(() => void map.showLens(port, lens));
	});

	const hrefFor = (lens: string) => {
		const q = data.branch ? `?branch=${encodeURIComponent(data.branch)}` : '';
		return `/org/${data.slug}/repositories/${data.repo.id}/diagrams/${lens}${q}`;
	};

	async function pickBranch(branch: string) {
		const url = new URL(page.url);
		url.searchParams.set('branch', branch);
		await goto(url, { invalidateAll: true });
	}
</script>

<svelte:head><title>{data.repo.name} diagrams · {data.orgName} · Dōjō</title></svelte:head>

<div class="flex h-full min-h-0 flex-col gap-3 p-4" data-testid="repo-diagrams" data-lens={data.lens}>
	<div class="flex flex-wrap items-center gap-3">
		<div>
			<h1 class="m-0 text-lg">{data.repo.name}</h1>
			<p class="m-0 font-mono text-xs text-ink-mute">{data.repo.key}</p>
		</div>
		<label class="ml-auto inline-flex items-center gap-2 text-sm">
			<span class="text-ink-mute">Branch</span>
			<select
				data-testid="branch-picker"
				value={data.branch ?? ''}
				onchange={(e) => void pickBranch(e.currentTarget.value)}
				disabled={data.repo.branches.length === 0}
			>
				{#if data.branch === null}
					<option value="">No tracked branch</option>
				{/if}
				{#each data.repo.branches as b (b.branch)}
					<option value={b.branch}>{b.branch}{b.is_default ? ' (default)' : ''}</option>
				{/each}
			</select>
		</label>
	</div>

	<nav class="flex flex-wrap gap-1" aria-label="Diagram lenses" data-testid="lens-nav">
		{#each LENSES as l (l.id)}
			<a
				href={hrefFor(l.id)}
				aria-current={l.id === data.lens ? 'page' : undefined}
				class={l.id === data.lens ? 'font-semibold' : ''}
			>
				{l.label}
			</a>
		{/each}
	</nav>

	{#if data.branch === null}
		<p data-testid="no-branch">This repository tracks no branch, so it shares no graph. Track one to see diagrams.</p>
	{:else}
		<div class="min-h-0 flex-1">
			{#key mapKey}
				<CodeMap {state} {port} label={`${data.repo.name} map`} />
			{/key}
		</div>
	{/if}
</div>
