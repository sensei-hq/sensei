<script lang="ts">
	import Icon from './Icon.svelte';
	import Chip from './Chip.svelte';
	import ClassChip from './ClassChip.svelte';
	import PhasePill from './PhasePill.svelte';
	import Spark from './Spark.svelte';
	import type { KitProject } from './types';

	// A project row (kit K2ProjectRow) — the list workhorse. `compact` is the
	// phone-friendly stacked row (name · repo · needs · phase · lastRun); the full
	// grid variant is the desktop table row (adds classification, dōjō, note,
	// sparkline). `showDojo` adds the owning-dōjō column to the grid. The grid uses
	// an inline `grid-template-columns` (geometry the utility scale doesn't model);
	// wrap it in an `overflow-x-auto` container for narrow widths.
	let {
		p,
		showDojo = true,
		compact = false,
		onopen
	}: {
		p: KitProject;
		showDojo?: boolean;
		compact?: boolean;
		onopen?: (p: KitProject) => void;
	} = $props();

	const cols = $derived(
		showDojo
			? '22px minmax(140px,1.3fr) 100px 108px minmax(0,1.7fr) 84px 36px 108px 34px'
			: '22px minmax(140px,1.3fr) 100px minmax(0,1.7fr) 84px 36px 108px 34px'
	);
</script>

{#if compact}
	<button
		type="button"
		onclick={() => onopen?.(p)}
		class="border-paper-edge flex w-full cursor-pointer items-center gap-3 border-b bg-transparent text-left"
		style="padding: 12px 16px"
	>
		<Icon name="folder" size={18} toneClass="text-accent" />
		<div style="min-width: 0; flex: 1">
			<div class="text-ink truncate text-sm font-medium" style="line-height: 1.2">{p.name}</div>
			<div class="mono text-ink-faint truncate text-xs" style="margin-top: 1px">{p.repo}</div>
		</div>
		{#if (p.needs ?? 0) > 0}
			<Chip
				icon="bell"
				toneClass="text-accent"
				softClass="bg-accent-soft"
				edgeClass="border-accent-soft">{p.needs}</Chip
			>
		{/if}
		<PhasePill phase={p.phase} />
		<span
			class="mono text-ink-faint flex-shrink-0 text-xs"
			style="width: 34px; text-align: right">{p.lastRun}</span
		>
	</button>
{:else}
	<button
		type="button"
		onclick={() => onopen?.(p)}
		class="border-paper-edge grid w-full cursor-pointer items-center border-b bg-transparent text-left"
		style="grid-template-columns: {cols}; gap: 12px; padding: 12px 16px"
	>
		<Icon name="folder" size={18} toneClass="text-accent" />
		<div style="min-width: 0">
			<div class="text-ink truncate text-sm font-medium" style="line-height: 1.2">{p.name}</div>
			<div class="mono text-ink-faint truncate text-xs" style="margin-top: 1px">{p.repo}</div>
		</div>
		<span class="flex items-center"><ClassChip kind={p.classification} /></span>
		{#if showDojo}
			<span class="text-ink-mute truncate text-xs">{p.dojoName || ''}</span>
		{/if}
		<span class="text-ink-mute truncate text-xs">{p.note || ''}</span>
		<span class="flex items-center" style="justify-content: flex-start">
			{#if p.spark}<Spark data={p.spark} />{/if}
		</span>
		<span class="flex items-center">
			{#if (p.needs ?? 0) > 0}
				<Chip
					icon="bell"
					toneClass="text-accent"
					softClass="bg-accent-soft"
					edgeClass="border-accent-soft">{p.needs}</Chip
				>
			{/if}
		</span>
		<span class="flex items-center justify-center"><PhasePill phase={p.phase} /></span>
		<span class="mono text-ink-faint text-xs" style="text-align: right">{p.lastRun}</span>
	</button>
{/if}
