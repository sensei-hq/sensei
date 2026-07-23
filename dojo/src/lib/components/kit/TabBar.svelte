<script lang="ts">
	import Icon from './Icon.svelte';
	import type { KitNavItem } from './types';

	// Mobile bottom tab bar (kit K2TabBar). Equal-width tabs, each an i-glyph icon
	// or a kanji glyph + label, with an optional count badge. Presentational:
	// selecting emits `onnav(id)`. The column count is driven by the tab count.
	let {
		tabs = [],
		active,
		onnav
	}: { tabs?: KitNavItem[]; active?: string; onnav?: (id: string) => void } = $props();
</script>

<div
	class="border-paper-edge bg-paper grid flex-shrink-0 border-t"
	style="grid-template-columns: repeat({tabs.length}, 1fr)"
>
	{#each tabs as it (it.id)}
		{@const on = active === it.id}
		<button
			type="button"
			onclick={() => onnav?.(it.id)}
			class="relative flex cursor-pointer flex-col items-center gap-1 bg-transparent {on
				? 'text-ink'
				: 'text-ink-mute'}"
			style="padding: 8px 4px 12px"
		>
			{#if it.icon}
				<Icon name={it.icon} size={20} toneClass={on ? 'text-accent' : 'text-ink-mute'} />
			{:else}
				<span class="kanji {on ? 'text-accent' : 'text-ink-mute'}" style="font-size: 17px"
					>{it.kanji}</span
				>
			{/if}
			<span class="text-xs {on ? 'font-semibold' : 'font-normal'}" style="white-space: nowrap"
				>{it.label}</span
			>
			{#if it.badge != null}
				<span
					class="mono bg-accent text-on-primary absolute rounded-full text-xs font-semibold"
					style="top: 4px; left: 50%; margin-left: 6px; padding: 0 6px; line-height: 14px"
					>{it.badge}</span
				>
			{/if}
		</button>
	{/each}
</div>
