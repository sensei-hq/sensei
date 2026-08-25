<script lang="ts">
	import { SectionHead, SubTabs, InboxRow, EmptyState, Banner } from '$lib/components/kit';
	import type { KitInbox, KitRun, KitNavItem } from '$lib/components/kit/types';
	import { filterInbox } from '$lib/relay-map';

	// The Inbox (mockup ScrInbox) — one list of every in-flight session, sorted
	// by what waits on you. Filter tabs (needs you · running · finished · all)
	// narrow it; a row opens the session detail (/you/runs/[run_id]), where
	// approve / decide / chat are answered — they are NOT surfaces of their own.
	// Degrades to an honest empty when nothing is in flight (DJ1).
	let {
		inbox = [],
		error = null,
		selectedId = null,
		onOpen
	}: {
		inbox?: KitInbox[];
		error?: string | null;
		selectedId?: string | null;
		onOpen: (run: KitRun) => void;
	} = $props();

	const FILTERS: KitNavItem[] = [
		{ id: 'needs', label: 'Needs you' },
		{ id: 'running', label: 'Running' },
		{ id: 'finished', label: 'Finished' },
		{ id: 'all', label: 'All' }
	];

	let filter = $state('needs');
	const shown = $derived(filterInbox(inbox, filter));
	const needTotal = $derived(inbox.reduce((n, r) => n + r.needs, 0));
</script>

{#snippet needBadge()}
	{#if needTotal > 0}<span class="mono text-accent text-xs">{needTotal} need you</span>{/if}
{/snippet}

<div class="flex flex-col p-4 gap-4 md:px-5 md:py-6">
	<SectionHead eyebrow="You · in flight" title="Inbox" count={inbox.length} right={needBadge} />
	<SubTabs tabs={FILTERS} active={filter} onPick={(id) => (filter = id)} />

	{#if error}
		<Banner tone="warning" title="Couldn't reach the dōjō">{error}</Banner>
	{/if}

	{#if !inbox.length}
		<EmptyState kanji="空" title="Nothing in flight.">
			Sessions you're running across your dōjōs show up here. When it's quiet, sensei stays quiet.
		</EmptyState>
	{:else if !shown.length}
		<EmptyState kanji="空" title="Nothing here.">No session matches that view right now.</EmptyState>
	{:else}
		<div class="border-paper-edge overflow-hidden rounded-lg border">
			{#each shown as row (row.run.id)}
				<InboxRow {row} selected={row.run.id === selectedId} {onOpen} />
			{/each}
		</div>
		<p class="mono text-ink-faint text-xs" style="line-height: 1.5">
			Sorted by what waits on you — then stalled or blocked, then running, then finished.
		</p>
	{/if}
</div>
