<script lang="ts">
	import Icon from './Icon.svelte';
	import Chip from './Chip.svelte';
	import type { KitRun } from './types';

	// A live-run card (kit K2RunCard) — task · project · session · assistant, with
	// a running/waiting status pill and an elapsed/edits meta line. `flat` drops the
	// card chrome so it reads as a row inside a single flush card (matching the
	// project list); `stacked` is the phone layout (title over meta over status).
	// `onOpen` fires with the run.
	let {
		run,
		flat = false,
		stacked = false,
		onOpen
	}: { run: KitRun; flat?: boolean; stacked?: boolean; onOpen?: (run: KitRun) => void } = $props();

	const live = $derived(run.state === 'running');
</script>

{#snippet statusPill()}
	<span
		class="inline-flex items-center gap-1 whitespace-nowrap rounded-full border text-xs {live
			? 'text-success bg-success-soft border-success-soft'
			: 'text-warning bg-warning-soft border-warning-soft'}"
		style="padding: 3px 10px"
	>
		<span class="rounded-full {live ? 'bg-success' : 'bg-warning'}" style="width: 6px; height: 6px"
		></span>{live ? 'running' : 'waiting'}
	</span>
{/snippet}

{#if stacked}
	<button
		type="button"
		onclick={() => onOpen?.(run)}
		class="border-paper-edge flex w-full cursor-pointer flex-col gap-2 border-b bg-transparent text-left"
		style="padding: 12px 16px"
	>
		<div class="flex items-center gap-2">
			<Icon name="eye" size={20} toneClass={live ? 'text-success' : 'text-warning'} />
			<span class="text-ink flex-1 text-sm font-medium" style="line-height: 1.3">{run.task}</span>
		</div>
		<div class="mono text-ink-faint text-xs">{run.project} · {run.id} · {run.assistant}</div>
		<div class="flex items-center gap-2">
			{@render statusPill()}
			{#if run.gate}
				<Chip
					icon="command"
					toneClass="text-accent"
					softClass="bg-accent-soft"
					edgeClass="border-accent-soft">gate</Chip
				>
			{/if}
			<span class="flex-1"></span>
			<span class="mono text-ink-mute text-xs">{run.elapsed} · {run.edits} edits</span>
		</div>
	</button>
{:else}
	<button
		type="button"
		onclick={() => onOpen?.(run)}
		class="flex w-full cursor-pointer items-center gap-4 bg-transparent text-left {flat
			? 'border-paper-edge border-b'
			: 'bg-paper-soft border-paper-edge rounded-lg border'}"
		style="padding: {flat ? '12px 16px' : '16px'}"
	>
		<Icon name="eye" size={22} toneClass={live ? 'text-success' : 'text-warning'} />
		<div class="flex-1" style="min-width: 0">
			<div class="flex items-center gap-2">
				<span class="text-ink text-sm font-medium">{run.task}</span>
				{#if run.gate}
					<Chip
						icon="command"
						toneClass="text-accent"
						softClass="bg-accent-soft"
						edgeClass="border-accent-soft">gate waiting</Chip
					>
				{/if}
			</div>
			<div class="mono text-ink-faint text-xs" style="margin-top: 2px">
				{run.project} · {run.id} · {run.assistant}
			</div>
		</div>
		<div class="flex flex-col items-end" style="gap: 3px">
			{@render statusPill()}
			<span class="mono text-ink-mute text-xs">{run.elapsed} · {run.edits} edits</span>
		</div>
	</button>
{/if}
