<script lang="ts">
	import Icon from './Icon.svelte';
	import Chip from './Chip.svelte';
	import type { KitRule } from './types';

	// A rule row (kit K2RuleRow) — a single rule on the ladder. When `onToggle` is
	// given it leads with an include checkbox (the pack-authoring surface);
	// otherwise it leads with the rule's kanji (the read-only ladder view).
	// `showLevel` reveals the rung the rule entered from — as a jump button when
	// `onJump` is given, else a static chip. `hard` rules carry the ★
	// non-negotiable marker; `onEdit` adds a trailing pencil.
	let {
		rule,
		included = true,
		showLevel = false,
		onToggle,
		onEdit,
		onJump
	}: {
		rule: KitRule;
		included?: boolean;
		showLevel?: boolean;
		onToggle?: (rule: KitRule) => void;
		onEdit?: (rule: KitRule) => void;
		onJump?: (rule: KitRule) => void;
	} = $props();
</script>

<div class="border-paper-edge flex items-center gap-3 border-b" style="padding: 8px 16px">
	{#if onToggle}
		<button
			type="button"
			onclick={() => onToggle(rule)}
			aria-pressed={included}
			aria-label={included ? 'Exclude rule' : 'Include rule'}
			class="flex-shrink-0 cursor-pointer rounded-sm border text-center {included
				? 'bg-accent border-accent text-on-primary'
				: 'border-ink-faint bg-transparent'}"
			style="width: 16px; height: 16px; font-size: 11px; line-height: 14px"
		>
			{included ? '✓' : ''}
		</button>
	{:else}
		<span
			class="kanji text-ink-mute flex-shrink-0 text-center"
			style="font-size: 13px; width: 16px">{rule.kanji}</span
		>
	{/if}
	<span
		class="text-ink flex-1 text-sm"
		style="line-height: 1.35; opacity: {included ? 1 : 0.5}">{rule.text}</span
	>
	{#if showLevel && rule.level}
		{#if onJump}
			<button
				type="button"
				onclick={() => onJump(rule)}
				title="Jump to {rule.level}"
				class="mono text-ink-mute bg-paper-mute border-paper-edge inline-flex cursor-pointer items-center gap-1 whitespace-nowrap rounded-full border text-xs"
				style="padding: 3px 9px"
			>
				{rule.level}<Icon name="arrow-right-up" size={12} toneClass="text-ink-mute" />
			</button>
		{:else}
			<Chip mono>{rule.level}</Chip>
		{/if}
	{/if}
	{#if rule.hard}
		<span class="text-accent inline-flex items-center gap-1 whitespace-nowrap text-xs">
			<span style="font-size: 12px">★</span>non-negotiable
		</span>
	{/if}
	{#if onEdit}
		<button
			type="button"
			onclick={() => onEdit(rule)}
			title="Edit rule"
			aria-label="Edit rule"
			class="flex flex-shrink-0 cursor-pointer items-center bg-transparent"
		>
			<Icon name="pen-2" size={15} toneClass="text-ink-mute" />
		</button>
	{/if}
</div>
