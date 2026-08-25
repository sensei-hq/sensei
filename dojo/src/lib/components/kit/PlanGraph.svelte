<script lang="ts">
	import type { KitPlan, KitTask } from './types';
	import { phases } from './plan';
	import { K2_NODE } from './vocab';
	import Icon from './Icon.svelte';
	import PlanStage from './PlanStage.svelte';

	// The whole plan as a graph (kit K2PlanGraph): stages flow top→bottom below
	// `md` and left→right (wrapping) from `md` up, arrows between; an optional
	// legend of the task states.
	let {
		plan,
		selectedId,
		onSelect,
		legend = true
	}: {
		plan: KitPlan;
		selectedId?: string;
		onSelect?: (task: KitTask) => void;
		legend?: boolean;
	} = $props();

	const stages = $derived(phases(plan));
</script>

<div class="flex flex-col gap-4">
	<div class="flex flex-col items-stretch md:flex-row md:flex-wrap md:gap-x-3 md:gap-y-6">
		{#each stages as s, i (s.id)}
			{#if i > 0}
				<!-- The flow arrow between stages: down the column below md, along the
				     row from md up. One icon rotated, rather than two icons behind a
				     branch. The 30px top offset lines it up with a stage's header. -->
				<div
					class="flex flex-shrink-0 justify-center py-1 md:items-center md:py-0 md:pt-[30px]"
				>
					<span class="flex rotate-90 md:rotate-0">
						<Icon name="alt-arrow-right" size={16} toneClass="text-ink-faint" />
					</span>
				</div>
			{/if}
			<PlanStage stage={s} index={i} {selectedId} {onSelect} />
		{/each}
	</div>
	{#if legend}
		<div class="border-paper-edge flex flex-wrap items-center gap-4 border-t pt-3">
			{#each Object.values(K2_NODE) as n (n.label)}
				<span class="flex items-center gap-1">
					<Icon name={n.icon} size={13} toneClass={n.text} />
					<span class="mono text-ink-mute text-xs">{n.label}</span>
				</span>
			{/each}
		</div>
	{/if}
</div>
