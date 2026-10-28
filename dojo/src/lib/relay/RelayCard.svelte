<script lang="ts">
	import type { RelaySession } from './types';
	import { sessionView } from '$lib/relay-view';

	// One inbox session card (mockup K2InboxRow). Pure: a RelaySession in, markup out.
	// Every decision — the dot, the why-line, the pips, the count — is sessionView's,
	// from the same tables the detail renders (audit 2026-10-28, item 5).
	let {
		session,
		selected = false,
		onopen
	}: { session: RelaySession; selected?: boolean; onopen?: (id: string) => void } = $props();

	const view = $derived(sessionView(session));
</script>

<div class="border-paper-edge border-b {selected ? 'bg-paper-mute' : ''}">
	<button
		type="button"
		onclick={() => onopen?.(session.id)}
		class="grid w-full cursor-pointer items-start gap-3 bg-transparent px-4 py-3 text-left"
		style="grid-template-columns: 10px minmax(0, 1fr)"
	>
		<span
			class="rounded-full {view.dotClass}"
			style="width: 7px; height: 7px; margin-top: 6px"
		></span>
		<span class="flex min-w-0 flex-col" style="gap: 3px">
			<span class="flex items-baseline gap-2">
				<span
					class="mono text-ink-soft min-w-0 text-xs"
					style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap"
					>{session.project ?? ''}</span
				>
				<span class="flex-1"></span>
				<span class="mono text-ink-faint shrink-0 text-xs">{view.age}</span>
			</span>
			<span
				class="text-sm {view.titleClass}"
				style="line-height: 1.35; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden"
				>{session.title}</span
			>
			<span class="flex items-center gap-2" style="margin-top: 1px">
				<span class="text-xs {view.whyClass}">{view.why}</span>
				<span class="flex-1"></span>
				{#if view.pips.length}
					<span class="flex items-center" style="gap: 3px" title="plan progress">
						{#each view.pips as p (p.id)}
							<span class="rounded-full {p.pipClass}" style="width: 12px; height: 5px"></span>
						{/each}
					</span>
				{/if}
				<span class="mono text-ink-faint shrink-0 text-xs">{view.count}</span>
			</span>
		</span>
	</button>
</div>
