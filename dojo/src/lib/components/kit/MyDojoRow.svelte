<script lang="ts">
	import Chip from './Chip.svelte';
	import RoleTag from './RoleTag.svelte';
	import { kindTone } from './vocab';
	import type { KitDojo } from './types';

	// A dōjō membership row (kit K2MyDojoRow) — org identity glyph · name · kind
	// chip · route/members/projects meta · optional "needs you" chip · your role ·
	// a go affordance.
	let { dojo, onopen }: { dojo: KitDojo; onopen?: (d: KitDojo) => void } = $props();

	const kind = $derived(kindTone(dojo.kind));
</script>

<button
	type="button"
	onclick={() => onopen?.(dojo)}
	class="border-paper-edge flex w-full cursor-pointer items-center gap-4 border-b bg-transparent text-left"
	style="padding: 16px"
>
	<span
		class="kanji flex-shrink-0 text-center {kind.text}"
		style="font-size: 26px; line-height: 1; width: 34px">{dojo.kanji}</span
	>
	<div class="flex-1" style="min-width: 0">
		<div class="flex items-center gap-2">
			<span class="text-ink text-base font-medium">{dojo.name}</span>
			<Chip mono>{dojo.kind}</Chip>
		</div>
		<div class="mono text-ink-faint text-xs" style="margin-top: 2px">
			{dojo.route}{#if dojo.members != null} · {dojo.members} members{/if}{#if dojo.projects != null}
				· {dojo.projects} projects{/if}
		</div>
	</div>
	{#if (dojo.needs ?? 0) > 0}
		<Chip icon="bell" toneClass="text-accent" softClass="bg-accent-soft" edgeClass="border-accent-soft"
			>{dojo.needs} need you</Chip
		>
	{/if}
	<RoleTag role={dojo.role} />
	<span class="text-ink-faint" style="font-size: 18px" aria-hidden="true">→</span>
</button>
