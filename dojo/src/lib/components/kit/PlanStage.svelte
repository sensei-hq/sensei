<script lang="ts">
	import type { KitPhase, KitTask } from './types';
	import { nodeTone } from './vocab';
	import { stageState } from './plan';
	import Icon from './Icon.svelte';
	import PlanNode from './PlanNode.svelte';

	// One phase column (kit K2PlanStage): a numbered header + roll-up dot, a
	// parallel/sequential indicator (from `stage.mode`, else derived from deps),
	// then the tasks wired as a fan (parallel) or an arrow chain (sequential).
	let {
		stage,
		index,
		selectedId,
		onSelect
	}: {
		stage: KitPhase;
		index: number;
		selectedId?: string;
		onSelect?: (task: KitTask) => void;
	} = $props();

	const tone = $derived(nodeTone(stageState(stage)));
	const parallel = $derived(
		stage.mode ? stage.mode === 'parallel' : stage.tasks.filter((t) => !t.deps?.length).length > 1
	);
	const num = $derived(String(index + 1).padStart(2, '0'));
</script>

<div class="flex min-w-0 flex-col" style="gap: var(--space-2); flex: 1 1 0; min-width: 150px">
	<div
		class="border-paper-edge flex items-center gap-2 border-b"
		style="padding-bottom: var(--space-2)"
	>
		<span class="mono text-ink-faint text-xs">{num}</span>
		<span class="text-ink min-w-0 flex-1 text-sm font-medium">{stage.title}</span>
		<span class="rounded-full {tone.fill}" style="width: 6px; height: 6px; flex-shrink: 0"></span>
	</div>
	<div class="flex items-center gap-1" style="margin-bottom: 2px">
		<Icon
			name={parallel ? 'transfer-horizontal' : 'arrow-right'}
			size={13}
			toneClass="text-ink-faint"
		/>
		<span class="mono text-ink-faint text-xs"
			>{parallel ? 'parallel · all at once' : 'sequential · in order'}</span
		>
	</div>
	{#if parallel}
		<div class="flex" style="gap: var(--space-2)">
			<span class="bg-paper-edge rounded-sm" style="width: 1px; flex-shrink: 0"></span>
			<div class="flex min-w-0 flex-1 flex-col" style="gap: var(--space-2)">
				{#each stage.tasks as t (t.id)}
					<PlanNode task={t} selected={selectedId === t.id} {onSelect} />
				{/each}
			</div>
		</div>
	{:else}
		<div class="flex flex-col">
			{#each stage.tasks as t, i (t.id)}
				{#if i > 0}
					<span class="flex items-center" style="height: 16px; padding-left: var(--space-4)">
						<Icon name="alt-arrow-down" size={13} toneClass="text-ink-faint" />
					</span>
				{/if}
				<PlanNode task={t} selected={selectedId === t.id} {onSelect} />
			{/each}
		</div>
	{/if}
</div>
