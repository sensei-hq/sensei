<script lang="ts">
	import type { KitActivity } from './types';
	import Icon from './Icon.svelte';

	// The run's activity feed (kit K2RunActivity): each entry an icon + text +
	// timestamp, newest first (the mapper sorts). A flush, hairline-separated list.
	let { feed = [] }: { feed?: KitActivity[] } = $props();
</script>

<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
	{#each feed as e, i (i)}
		<div
			class="flex items-center gap-3 {i < feed.length - 1 ? 'border-paper-edge border-b' : ''}"
			style="padding: var(--space-3) var(--space-4)"
		>
			<Icon name={e.icon} size={16} toneClass={e.toneClass} />
			<span class="text-ink-soft min-w-0 flex-1 text-sm">{e.text}</span>
			<span class="mono text-ink-faint text-xs">{e.at}</span>
		</div>
	{/each}
</div>
