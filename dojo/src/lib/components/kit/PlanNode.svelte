<script lang="ts">
	import type { KitTask } from './types';
	import { nodeTone } from './vocab';
	import Icon from './Icon.svelte';

	// One task node (kit K2PlanNode): a state-tinted icon + title + optional
	// summary in a selectable button. The bg/border come from the state's tone;
	// a selected node highlights to paper-mute.
	let {
		task,
		selected = false,
		onSelect
	}: { task: KitTask; selected?: boolean; onSelect?: (task: KitTask) => void } = $props();

	const tone = $derived(nodeTone(task.state));
</script>

<button
	type="button"
	onclick={() => onSelect?.(task)}
	class="flex w-full items-start gap-2 rounded border text-left {tone.dashed
		? 'border-dashed'
		: ''} {tone.edge} {selected ? 'bg-paper-mute' : tone.soft}"
	style="padding: var(--space-2) var(--space-3)"
>
	<Icon name={tone.icon} size={16} toneClass={tone.text} />
	<span class="min-w-0 flex-1">
		<span
			class="block text-sm {task.state === 'pending' || task.state === 'skipped'
				? 'text-ink-mute'
				: 'text-ink'}"
			style="line-height: 1.35">{task.title}</span
		>
		{#if task.summary}<span class="mono text-ink-faint block text-xs" style="margin-top: 2px"
				>{task.summary}</span
			>{/if}
	</span>
</button>
