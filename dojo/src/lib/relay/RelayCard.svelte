<script lang="ts">
	import type { RelaySession, SegmentState } from './types';
	import { relativeAge } from '$lib/triage/view';
	import * as m from '$lib/paraglide/messages';

	// One inbox session card (mockup K2InboxRow). Pure: a RelaySession in, markup out.
	// Owns its own styling so mockup-fidelity is verified here, with a mock prop. The
	// list row: status dot · project · age · 2-line title · why-line · plan-pips ·
	// done/total. Accent when it needs you; paper-mute when selected.
	let {
		session,
		selected = false,
		onopen
	}: { session: RelaySession; selected?: boolean; onopen?: (id: string) => void } = $props();

	// Status dot: accent when it needs you, the attention/running fill otherwise, else
	// a hollow hairline dot (mirrors the mockup's dot logic).
	const dotClass = $derived(
		session.needs > 0
			? 'bg-accent'
			: session.attention === 'stalled'
				? 'bg-warning'
				: session.attention === 'blocked' || session.attention === 'failed'
					? 'bg-danger'
					: session.status === 'running'
						? 'bg-success'
						: ''
	);

	const statusLabels: Record<RelaySession['status'], string> = {
		running: m.status_running(),
		stalled: m.status_stalled(),
		blocked: m.status_blocked(),
		paused: m.status_paused(),
		done: m.status_done(),
		failed: m.status_failed()
	};

	// The why-line — what waits on you, then attention, else the status label.
	const why = $derived(
		session.needs > 0
			? session.needs === 1
				? m.card_needs_one()
				: m.card_needs_many({ count: session.needs })
			: session.attention === 'stalled'
				? m.card_no_heartbeat()
				: session.attention === 'blocked'
					? m.card_blocked()
					: session.attention === 'failed'
						? m.card_failed()
						: statusLabels[session.status]
	);
	const whyClass = $derived(
		session.needs > 0
			? 'text-accent font-semibold'
			: session.attention === 'stalled'
				? 'text-warning font-semibold'
				: session.attention
					? 'text-danger font-semibold'
					: 'text-ink-mute'
	);

	const age = $derived(session.lastEventAt ? relativeAge(session.lastEventAt) : '');

	// One pip per phase, tinted by its roll-up state; pending/skipped is hollow.
	function pipClass(state: SegmentState): string {
		switch (state) {
			case 'done':
				return 'bg-success';
			case 'active':
				return 'bg-accent';
			case 'blocked':
			case 'failed':
				return 'bg-danger';
			case 'needs_review':
				return 'bg-warning';
			default:
				return 'border-paper-edge border bg-transparent';
		}
	}
</script>

<div class="border-paper-edge border-b {selected ? 'bg-paper-mute' : ''}">
	<button
		type="button"
		onclick={() => onopen?.(session.id)}
		class="grid w-full cursor-pointer items-start gap-3 bg-transparent px-4 py-3 text-left"
		style="grid-template-columns: 10px minmax(0, 1fr)"
	>
		<span
			class="rounded-full {dotClass || 'border-ink-faint border'}"
			style="width: 7px; height: 7px; margin-top: 6px"
		></span>
		<span class="flex min-w-0 flex-col" style="gap: 3px">
			<span class="flex items-baseline gap-2">
				<span
					class="mono text-ink-mute min-w-0 text-xs"
					style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap"
					>{session.project ?? ''}</span
				>
				<span class="flex-1"></span>
				<span class="mono text-ink-faint shrink-0 text-xs">{age}</span>
			</span>
			<span
				class="text-sm {session.status === 'done' ? 'text-ink-mute' : 'text-ink font-medium'}"
				style="line-height: 1.35; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden"
				>{session.title}</span
			>
			<span class="flex items-center gap-2" style="margin-top: 1px">
				<span class="text-xs {whyClass}">{why}</span>
				<span class="flex-1"></span>
				{#if session.plan.phases.length}
					<span class="flex items-center" style="gap: 3px" title="plan progress">
						{#each session.plan.phases as p (p.id)}
							<span class="rounded-full {pipClass(p.state)}" style="width: 12px; height: 5px"></span>
						{/each}
					</span>
				{/if}
				<span class="mono text-ink-faint shrink-0 text-xs">{session.done}/{session.total}</span>
			</span>
		</span>
	</button>
</div>
