<script lang="ts">
	import { KIND_ICONS } from '@sensei/glyphs';
	import KanjiToken from './KanjiToken.svelte';
	import Chip from './Chip.svelte';
	import RoleTag from './RoleTag.svelte';
	import { kindTone } from './vocab';
	import type { KitDojo } from './types';

	// A dōjō membership row (kit K2MyDojoRow) — org identity glyph · name · kind
	// chip · route/members/workspaces meta · optional "needs you" chip · your role ·
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
	<!-- A dōjō's own mark: an identity, kanji in both modes (#133). -->
	<KanjiToken char={dojo.kanji} icon={KIND_ICONS.dojo} size="2xl" toneClass={kind.text} w={34} />
	<div class="flex-1" style="min-width: 0">
		<div class="flex items-center gap-2">
			<span class="text-ink text-base font-medium">{dojo.name}</span>
			<Chip mono>{dojo.kind}</Chip>
		</div>
		<div class="mono text-ink-faint text-xs" style="margin-top: 2px">
			{dojo.route}{#if dojo.members != null} · {dojo.members} members{/if}{#if dojo.workspaces != null}
				· {dojo.workspaces} workspaces{/if}
		</div>
	</div>
	{#if (dojo.needs ?? 0) > 0}
		<Chip icon="bell" toneClass="text-accent" softClass="bg-accent-soft" edgeClass="border-accent-soft"
			>{dojo.needs} need you</Chip
		>
	{/if}
	<RoleTag role={dojo.role} />
	<span class="text-ink-faint text-lg" aria-hidden="true">→</span>
</button>
