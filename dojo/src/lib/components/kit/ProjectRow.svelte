<script lang="ts">
	import Icon from './Icon.svelte';
	import Chip from './Chip.svelte';
	import ClassChip from './ClassChip.svelte';
	import PhasePill from './PhasePill.svelte';
	import Spark from './Spark.svelte';
	import type { KitProject } from './types';

	// A project row (kit K2ProjectRow) — the list workhorse, one row at every width.
	// Below `md` it is a flex row (icon · name/repo · needs · phase · lastRun);
	// from `md` up it becomes the table row, and the four extra cells
	// (classification, dōjō, note, sparkline) un-hide into the grid. The shared
	// cells appear in the same relative order in both, which is what lets one
	// markup serve both instead of two near-identical blocks behind a `compact`
	// flag. `showDojo` adds the owning-dōjō column. `grid-template-columns` is
	// inline because the track list is dynamic geometry the utility scale doesn't
	// model; it is inert below `md`, where the row is flex.
	let {
		p,
		showDojo = true,
		onopen
	}: {
		p: KitProject;
		showDojo?: boolean;
		onopen?: (p: KitProject) => void;
	} = $props();

	const cols = $derived(
		showDojo
			? '22px minmax(140px,1.3fr) 100px 108px minmax(0,1.7fr) 84px 36px 108px 34px'
			: '22px minmax(140px,1.3fr) 100px minmax(0,1.7fr) 84px 36px 108px 34px'
	);
</script>

<button
	type="button"
	onclick={() => onopen?.(p)}
	class="border-paper-edge flex w-full cursor-pointer items-center gap-3 border-b bg-transparent px-4 py-3 text-left md:grid"
	style="grid-template-columns: {cols}"
>
	<Icon name="folder" size={18} toneClass="text-accent" />
	<div class="min-w-0 flex-1">
		<div class="text-ink truncate text-sm font-medium" style="line-height: 1.2">{p.name}</div>
		<div class="mono text-ink-faint truncate text-xs" style="margin-top: 1px">{p.repo}</div>
	</div>
	<span class="hidden items-center md:flex"><ClassChip kind={p.classification} /></span>
	{#if showDojo}
		<span class="text-ink-mute hidden truncate text-xs md:block">{p.dojoName || ''}</span>
	{/if}
	<span class="text-ink-mute hidden truncate text-xs md:block">{p.note || ''}</span>
	<span class="hidden items-center justify-start md:flex">
		{#if p.spark}<Spark data={p.spark} />{/if}
	</span>
	<span class="flex flex-shrink-0 items-center">
		{#if (p.needs ?? 0) > 0}
			<Chip
				icon="bell"
				toneClass="text-accent"
				softClass="bg-accent-soft"
				edgeClass="border-accent-soft">{p.needs}</Chip
			>
		{/if}
	</span>
	<span class="flex flex-shrink-0 items-center justify-center"><PhasePill phase={p.phase} /></span>
	<span class="mono text-ink-faint w-[34px] flex-shrink-0 text-right text-xs">{p.lastRun}</span>
</button>
