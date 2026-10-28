<script lang="ts">
	import KanjiToken from './KanjiToken.svelte';
	import Chip from './Chip.svelte';
	import RuleRow from './RuleRow.svelte';
	import type { KitLadderRung } from './types';

	// One rung of the constitution ladder (kit K2LadderRung) — a scope identity
	// (kanji · scope · name · caption) with its rules and lock count. `active`
	// highlights the selected rung; `onSelect` fires with the rung id. `showRules`
	// (default true) reveals the rule list under the header. The rule packs applied
	// at the rung (a stack's pack sits on the repository where the stack is
	// detected — D29 · D33) show as chips under the header; a rung with none shows
	// no pack row. `rung.tone === 'accent'` tints the rung, otherwise neutral ink.
	let {
		rung,
		active,
		showRules = true,
		onSelect
	}: {
		rung: KitLadderRung;
		active?: string;
		showRules?: boolean;
		onSelect?: (id: string) => void;
	} = $props();

	const on = $derived(active === rung.id);
	const toneClass = $derived(rung.tone === 'accent' ? 'text-accent' : 'text-ink-mute');
	const rules = $derived(rung.rules ?? []);
	const locks = $derived(rules.filter((r) => r.hard).length);
	const packs = $derived(rung.packs ?? []);
</script>

<div
	class="overflow-hidden rounded-lg border {on
		? 'border-accent bg-accent-soft'
		: 'border-paper-edge bg-paper-soft'}"
>
	<button
		type="button"
		onclick={() => onSelect?.(rung.id)}
		class="flex w-full cursor-pointer items-center gap-3 bg-transparent text-left"
		style="padding: 12px 16px"
	>
		<KanjiToken char={rung.kanji} size="xl" {toneClass} w={28} />
		<div class="flex-1" style="min-width: 0">
			<div class="flex items-center gap-2">
				<span class="text-xs font-semibold uppercase {toneClass}" style="letter-spacing: 0.18em"
					>{rung.scope}</span
				>
				<span class="text-ink text-sm font-medium">{rung.name}</span>
			</div>
			<div class="mono text-ink-soft text-xs" style="margin-top: 1px">{rung.caption}</div>
		</div>
		<span class="mono text-ink-faint text-xs">{rules.length} rules</span>
		{#if locks > 0}
			<Chip
				icon="lock-keyhole"
				toneClass="text-accent"
				softClass="bg-accent-soft"
				edgeClass="border-accent-soft">{locks} locked</Chip
			>
		{/if}
	</button>
	{#if packs.length}
		<div class="border-paper-edge flex flex-wrap items-center gap-2 border-t" style="padding: 8px 16px">
			<span class="mono text-ink-soft text-xs">packs applied here</span>
			{#each packs as pack (pack)}
				<Chip icon="box" toneClass="text-on-accent-soft" softClass="bg-accent-soft" edgeClass="border-accent-soft"
					>{pack}</Chip
				>
			{/each}
		</div>
	{/if}
	{#if showRules}
		<div class="border-paper-edge bg-paper border-t">
			{#each rules as rule, i (i)}
				<RuleRow {rule} />
			{/each}
		</div>
	{/if}
</div>
