<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { untrack } from 'svelte';
	import { CodeMap, CodeMapState, type Lens } from '@sensei/codemap';
	import { graphPortClient } from '$lib/graph-port-client';
	import { GraphBranchesState } from '$lib/graph-branches-state.svelte';
	import { getGraphSchema, setGraphBranch } from '$lib/client-data';
	import { GRAPH_LABELS } from '$lib/graph-labels';
	import { GraphSchemaState } from '$lib/graph-schema-state.svelte';
	import { toErGraph } from '$lib/er-graph';
	import { designTabs, EntitiesView, EnumView, ObjectView, OverviewView, TableView, toGraphInput } from '@rokkit/graph/schema';
	import { ErDiagram, Neighborhood } from '@rokkit/graph';
	import { Tabs } from '@rokkit/ui';
	import { messages } from '@rokkit/states';
	import { Notice } from '@sensei/ui';
	import type { DiagramLens } from './+page.js';

	// One repository's diagrams (#268): the shared Code map over the dōjō's
	// graph reads, with the branch picker and the lens switcher. The map's
	// state is keyed by repository + branch, so switching branch re-reads
	// everything instead of showing the old branch's picture. Untracked lens
	// switching, as the app's page: drilling in must not climb back out.
	let { data } = $props();

	const LENSES: { id: DiagramLens; label: string }[] = [
		{ id: 'structure', label: 'Structure' },
		{ id: 'wheel', label: 'Wheel' },
		{ id: 'zones', label: 'Zones' },
		{ id: 'matrix', label: 'Matrix' },
		{ id: 'cycles', label: 'Cycles' },
		{ id: 'coupling', label: 'Coupling' },
		{ id: 'complexity', label: 'Complexity' },
		{ id: 'schema', label: 'Database · ER' }
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
		if (data.lens === 'schema') return;
		const [map, lens] = [state, data.lens as Lens];
		untrack(() => void map.showLens(port, lens));
	});

	// The schema ER view reads its own payload, keyed by repository + branch
	// like the map: switching branch re-reads the design.
	const schemaApi = {
		tryGetSchema: async () => {
			try {
				const payload = await getGraphSchema({ repoKey: data.repo.key, branch: data.branch ?? '', fetch, accessToken: data.accessToken });
				return { ok: true as const, data: payload as unknown as Record<string, unknown> };
			} catch (e) {
				const err = e as { status?: number; message?: string };
				return { ok: false as const, error: { status: typeof err.status === 'number' ? err.status : 0, message: err.message ?? 'the request failed' } };
			}
		}
	};
	let schemaCurrent: { key: string; view: GraphSchemaState } | null = null;
	const schema = $derived.by(() => {
		if (schemaCurrent?.key !== mapKey) schemaCurrent = { key: mapKey, view: new GraphSchemaState(schemaApi) };
		return (schemaCurrent as { view: GraphSchemaState }).view;
	});
	$effect(() => {
		if (data.lens !== 'schema') return;
		void mapKey;
		untrack(() => void schema.load());
	});
	const erGraph = $derived(schema.model ? toErGraph(schema.model) : null);
	const erEntities = $derived(schema.model ? toGraphInput(schema.model, 'er') : null);

	const hrefFor = (lens: string) => {
		const q = data.branch ? `?branch=${encodeURIComponent(data.branch)}` : '';
		return `/org/${data.slug}/repositories/${data.repo.id}/diagrams/${lens}${q}`;
	};

	async function pickBranch(branch: string) {
		const url = new URL(page.url);
		url.searchParams.set('branch', branch);
		await goto(url, { invalidateAll: true });
	}

	// Tracked branches, for whoever may configure the repository (#268 §4).
	// The picker reads this same list, so tracking reflects immediately;
	// untracking the viewed branch moves the view to the tracked default.
	// Derived by repository: navigating repos resets the draft and list.
	const tracked = $derived.by(
		() =>
			new GraphBranchesState(data.repo.key, data.repo.branches, (k, b, t, l) =>
				setGraphBranch(k, b, t, l, { fetch, accessToken: data.accessToken })
			)
	);

	async function confirmUntrack() {
		const gone = tracked.pendingUntrack;
		await tracked.confirmUntrack();
		if (gone !== null && gone === data.branch && tracked.error === null) {
			const fallback =
				tracked.branches.find((b) => b.is_default)?.branch ?? tracked.branches[0]?.branch ?? null;
			if (fallback !== null && fallback !== gone) await pickBranch(fallback);
			else await goto(page.url, { invalidateAll: true });
		}
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
				disabled={tracked.branches.length === 0}
			>
				{#if data.branch === null}
					<option value="">No tracked branch</option>
				{/if}
				{#each tracked.branches as b (b.branch)}
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

	{#if data.repo.configurable}
		<details data-testid="branch-manager">
			<summary>Tracked branches</summary>
			<ul>
				{#each tracked.branches as b (b.branch)}
					<li>
						<span class="font-mono">{b.branch}</span>{b.is_default ? ' (default)' : ''}
						{#if tracked.pendingUntrack === b.branch}
							<span>Untracking deletes this branch's graph.</span>
							<button type="button" onclick={() => void confirmUntrack()} disabled={tracked.working}>Delete it</button>
							<button type="button" onclick={() => tracked.cancelUntrack()}>Keep</button>
						{:else}
							<button type="button" onclick={() => tracked.askUntrack(b.branch)}>Untrack</button>
						{/if}
					</li>
				{/each}
			</ul>
			<form
				onsubmit={(e) => {
					e.preventDefault();
					void tracked.track();
				}}
			>
				<input data-testid="track-branch" placeholder="Branch to track" bind:value={tracked.draft.branch} aria-label="Branch to track" />
				<select bind:value={tracked.draft.label} aria-label="Environment label">
					<option value="">No label</option>
					{#each GRAPH_LABELS as l (l)}
						<option value={l}>{l}</option>
					{/each}
				</select>
				<button type="submit" disabled={tracked.working}>Track</button>
			</form>
			{#if tracked.problem}
				<Notice size="inline" title={tracked.problem} />
			{/if}
			{#if tracked.error}
				<Notice size="inline" title={tracked.error} />
			{/if}
		</details>
	{/if}

	{#if data.branch === null}
		<p data-testid="no-branch">This repository tracks no branch, so it shares no graph. Track one to see diagrams.</p>
	{:else if data.lens === 'schema'}
		<div class="min-h-0 flex-1" data-testid="schema-screen">
			{#if schema.view === 'loading'}
				<p class="text-sm text-ink-soft" data-testid="schema-loading">Reading the branch schema.</p>
			{:else if schema.view === 'error'}
				<div class="rounded border border-warning p-4" data-testid="schema-error">
					<p class="text-sm text-warning">{schema.failure}</p>
					<button type="button" class="mt-2 cursor-pointer rounded border border-paper-edge bg-transparent px-3 py-1 text-sm" data-testid="schema-retry" onclick={() => schema.load()}>Retry</button>
				</div>
			{:else if schema.view === 'none'}
				<p class="text-sm text-ink-soft" data-testid="schema-none">No database structure is described on this branch.</p>
			{:else if schema.model}
				{@const model = schema.model}
				{#if schema.page && schema.selected}
					<nav class="flex items-center gap-2 text-sm">
						<button class="cursor-pointer border-none bg-transparent p-0 text-ink-mute hover:underline" onclick={() => schema.select(null)} data-testid="schema-back">‹ {data.repo.name}</button>
					</nav>
					<div data-testid="schema-page" data-page={schema.page}>
						{#if schema.page === 'table'}
							<TableView {model} entityKey={schema.selected} onselect={(k) => schema.select(k)} />
						{:else if schema.page === 'object'}
							<ObjectView {model} entityKey={schema.selected} onselect={(k) => schema.select(k)} />
						{:else}
							<EnumView {model} entityKey={schema.selected} onselect={(k) => schema.select(k)} />
						{/if}
					</div>
				{:else}
					<div data-testid="schema-tabs">
						<Tabs options={designTabs()} value={schema.tab} name={messages.graph.designTabs} tabPanel={undefined} onchange={(v) => (schema.tab = String(v) as 'overview' | 'diagram' | 'entities')} />
					</div>
					{#if schema.tab === 'overview'}
						<OverviewView {model} counts={false} />
					{:else if schema.tab === 'diagram' && erGraph}
						<div class="h-[70vh] min-w-0 shrink-0" data-testid="schema-diagram">
							{#if schema.focus}
								<Neighborhood
									nodes={erGraph.nodes}
									edges={erGraph.edges}
									fields={erGraph.fields}
									focus={schema.focus}
									depth={1}
									bind:edgeStyle={schema.edgeStyle}
									controls
									legend
									label="Neighbours"
									onselect={(id) => id && id !== schema.focus && schema.focusOn(id)}
								/>
							{:else}
								<ErDiagram
									nodes={erGraph.nodes}
									edges={erGraph.edges}
									fields={erGraph.fields}
									bind:density={schema.density}
									bind:edgeStyle={schema.edgeStyle}
									controls
									legend
									label="Tables, views and what they reach"
									onselect={(id) => id && schema.focusOn(id)}
								/>
							{/if}
						</div>
					{:else if erEntities}
						<EntitiesView nodes={erEntities.nodes} edges={erEntities.edges} fields={erEntities.fields} />
					{/if}
				{/if}
			{/if}
		</div>
	{:else}
		<div class="min-h-0 flex-1">
			{#key mapKey}
				<CodeMap {state} {port} label={`${data.repo.name} map`} />
			{/key}
		</div>
	{/if}
</div>
