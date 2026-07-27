<script lang="ts">
	import type { KitNavItem } from './types';
	import Icon from './Icon.svelte';

	// In-page filter/section tabs (kit K2SubTabs). Data-driven off KitNavItem
	// (id · label · optional icon · optional badge); the active tab fills, the
	// rest are muted. Pill padding is geometry (inline); colors are tokens.
	let {
		tabs,
		active,
		onPick
	}: { tabs: KitNavItem[]; active: string; onPick: (id: string) => void } = $props();
</script>

<div class="flex flex-wrap items-center gap-1" role="tablist">
	{#each tabs as t (t.id)}
		{@const on = t.id === active}
		<button
			type="button"
			role="tab"
			aria-selected={on}
			onclick={() => onPick(t.id)}
			class="inline-flex items-center gap-1 whitespace-nowrap rounded-full text-sm {on
				? 'bg-paper-mute text-ink font-medium'
				: 'text-ink-mute'}"
			style="padding: 4px 12px"
		>
			{#if t.icon}<Icon name={t.icon} size={14} toneClass={on ? 'text-ink' : 'text-ink-mute'} />{/if}
			{t.label}
			{#if t.badge}<span class="mono text-accent text-xs">{t.badge}</span>{/if}
		</button>
	{/each}
</div>
