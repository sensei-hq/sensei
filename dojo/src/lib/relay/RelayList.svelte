<script lang="ts">
	import { SectionHead, SubTabs, EmptyState } from '$lib/components/kit';
	import type { KitNavItem } from '$lib/components/kit/types';
	import RelayCard from './RelayCard.svelte';
	import { relayInboxState } from './relay-inbox-state.svelte';
	import type { RelayFilter } from './types';
	import * as m from '$lib/paraglide/messages';

	// The inbox list rail (mockup ScrInbox left panel). Pure presentation over
	// relayInboxState: header + filter tabs + a card per shown session, routing
	// selection back through the state. Realtime lives in the state, not here.
	const st = relayInboxState;

	const FILTERS: KitNavItem[] = [
		{ id: 'needs', label: m.filter_needs() },
		{ id: 'running', label: m.filter_running() },
		{ id: 'finished', label: m.filter_finished() },
		{ id: 'all', label: m.filter_all() }
	];
</script>

<div class="flex flex-col gap-4 px-5 py-6">
	<SectionHead eyebrow={m.inbox_eyebrow()} title={m.inbox_title()} count={st.sessions.length}>
		{#snippet right()}
			{#if st.needsCount > 0}
				<span class="mono text-accent text-xs">{m.inbox_needs_badge({ count: st.needsCount })}</span>
			{/if}
		{/snippet}
	</SectionHead>

	<SubTabs tabs={FILTERS} active={st.filter} onPick={(id) => st.setFilter(id as RelayFilter)} />

	{#if !st.sessions.length}
		<EmptyState kanji="空" title={m.inbox_empty_title()}>{m.inbox_empty_body()}</EmptyState>
	{:else if !st.shown.length}
		<EmptyState kanji="空" title={m.inbox_empty_title()}>{m.inbox_empty_filter()}</EmptyState>
	{:else}
		<div class="border-paper-edge overflow-hidden rounded-lg border">
			{#each st.shown as s (s.id)}
				<RelayCard session={s} selected={s.id === st.selectedId} onopen={(id) => st.select(id)} />
			{/each}
		</div>
		<p class="mono text-ink-faint text-xs" style="line-height: 1.5">{m.inbox_sort_hint()}</p>
	{/if}
</div>
