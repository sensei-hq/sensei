<script lang="ts">
	import type { KitPlan, KitTask } from './types';
	import { phases } from './plan';
	import { K2_NODE } from './vocab';
	import Icon from './Icon.svelte';
	import PlanStage from './PlanStage.svelte';

	// The whole plan as a graph (kit K2PlanGraph): stages flow left→right (wrapping
	// on desktop), top→bottom on mobile, arrows between; an optional legend of the
	// task states.
	let {
		plan,
		mobile = false,
		selectedId,
		onSelect,
		legend = true
	}: {
		plan: KitPlan;
		mobile?: boolean;
		selectedId?: string;
		onSelect?: (task: KitTask) => void;
		legend?: boolean;
	} = $props();

	const stages = $derived(phases(plan));
</script>

<div class="flex flex-col" style="gap: var(--space-4)">
	<div
		class={mobile ? 'flex flex-col' : 'flex flex-wrap'}
		style="align-items: stretch; {mobile ? '' : 'gap: var(--space-3); row-gap: var(--space-5)'}"
	>
		{#each stages as s, i (s.id)}
			{#if i > 0}
				<div
					class="flex {mobile ? 'justify-center' : 'items-center'}"
					style={mobile ? 'padding: var(--space-1) 0' : 'padding-top: 30px; flex-shrink: 0'}
				>
					<Icon
						name={mobile ? 'alt-arrow-down' : 'alt-arrow-right'}
						size={16}
						toneClass="text-ink-faint"
					/>
				</div>
			{/if}
			<PlanStage stage={s} index={i} {selectedId} {onSelect} />
		{/each}
	</div>
	{#if legend}
		<div
			class="border-paper-edge flex flex-wrap items-center gap-4 border-t"
			style="padding-top: var(--space-3)"
		>
			{#each Object.values(K2_NODE) as n (n.label)}
				<span class="flex items-center gap-1">
					<Icon name={n.icon} size={13} toneClass={n.text} />
					<span class="mono text-ink-mute text-xs">{n.label}</span>
				</span>
			{/each}
		</div>
	{/if}
</div>
