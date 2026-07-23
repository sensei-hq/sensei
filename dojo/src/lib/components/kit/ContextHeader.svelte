<script lang="ts">
	import KanjiToken from './KanjiToken.svelte';
	import RoleTag from './RoleTag.svelte';
	import { kindTone } from './vocab';
	import type { KitOrg, KitMe } from './types';

	// Context header (kit K2ContextHeader) — the band under the top bar that says
	// WHERE you are. Personal ("you") vs an org; this is the only chrome piece that
	// changes shape. The org band carries the accent top-rule that pairs with the
	// TopBar's org treatment.
	let {
		context = 'you',
		org,
		me
	}: { context?: 'you' | 'org'; org?: KitOrg; me?: KitMe } = $props();
</script>

{#if context === 'you'}
	<div
		class="border-paper-edge bg-paper-soft flex flex-shrink-0 items-center gap-3 border-b px-4"
		style="height: 46px"
	>
		<KanjiToken char="携" size="lg" toneClass="text-accent" />
		<span class="text-ink text-sm font-medium">Your work</span>
		<span class="text-ink-mute hidden text-sm md:inline"
			>— everything in flight, across every dōjō</span
		>
		<span class="flex-1"></span>
		<span class="mono text-ink-mute text-xs">{me?.name}</span>
	</div>
{:else if org}
	<div
		class="border-paper-edge border-t-accent bg-paper-soft flex flex-shrink-0 items-center gap-3 border-b border-t-2 px-4"
		style="height: 46px"
	>
		<KanjiToken char={org.kanji} size="lg" toneClass={kindTone(org.kind).text} />
		<span class="display text-lg" style="letter-spacing: -0.01em; white-space: nowrap"
			>{org.name}</span
		>
		<span
			class="mono text-ink-mute bg-paper border-paper-edge hidden whitespace-nowrap rounded-full border text-xs md:inline"
			style="padding: 3px 10px">{org.route}</span
		>
		<span class="flex-1"></span>
		<RoleTag role={org.role} />
	</div>
{/if}
