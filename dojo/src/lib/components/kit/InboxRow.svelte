<script lang="ts">
	import type { KitInbox, KitRun } from './types';
	import { statusTone } from './vocab';
	import PlanPips from './PlanPips.svelte';

	// One inbox row (kit K2InboxRow): a status dot, project · last-activity, a
	// 2-line task, the why-surfaced line, plan pips, and done/total. Accent when
	// it needs you. The 2-line clamp + truncate are geometry (inline); colors are
	// token classes.
	let {
		row,
		selected = false,
		onOpen
	}: { row: KitInbox; selected?: boolean; onOpen: (run: KitRun) => void } = $props();

	const tone = $derived(statusTone(row.status));
	const needsYou = $derived(row.needs > 0);
	const why = $derived(
		needsYou
			? `${row.needs} need${row.needs === 1 ? 's' : ''} you`
			: row.attention === 'stalled'
				? 'no heartbeat'
				: row.attention === 'blocked'
					? 'blocked on a task'
					: row.attention === 'failed'
						? 'a task failed'
						: null
	);
	const whyClass = $derived(needsYou ? 'text-accent' : row.attention ? tone.text : 'text-ink-mute');
	// The status dot: accent when it needs you, the status fill for an attention
	// state or a running run, else a hollow hairline dot.
	const dotFill = $derived(
		needsYou ? 'bg-accent' : row.attention ? tone.fill : row.status === 'running' ? 'bg-success' : ''
	);
</script>

<div class="border-b border-paper-edge {selected ? 'bg-paper-mute' : ''}">
	<button
		type="button"
		onclick={() => onOpen(row.run)}
		class="grid w-full items-start gap-3 text-left"
		style="grid-template-columns: 10px minmax(0, 1fr); padding: var(--space-3) var(--space-4)"
	>
		<span
			class="rounded-full {dotFill || 'border border-ink-faint'}"
			style="width: 7px; height: 7px; margin-top: 6px"
		></span>
		<span class="flex min-w-0 flex-col" style="gap: 3px">
			<span class="flex items-baseline gap-2">
				<span
					class="mono text-ink-mute min-w-0 text-xs"
					style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">{row.run.project}</span
				>
				<span class="flex-1"></span>
				<span class="mono text-ink-faint shrink-0 text-xs">{row.run.last ?? ''}</span>
			</span>
			<span
				class="text-sm {row.status === 'done' ? 'text-ink-mute' : 'text-ink font-medium'}"
				style="line-height: 1.35; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden"
				>{row.run.task}</span
			>
			<span class="flex items-center gap-2" style="margin-top: 1px">
				<span class="text-xs {whyClass} {needsYou || row.attention ? 'font-semibold' : ''}"
					>{why ?? tone.label}</span
				>
				<span class="flex-1"></span>
				{#if row.run.plan}<PlanPips plan={row.run.plan} caption={false} />{/if}
				<span class="mono text-ink-faint shrink-0 text-xs">{row.done}/{row.total}</span>
			</span>
		</span>
	</button>
</div>
