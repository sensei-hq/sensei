<script lang="ts">
	import { SectionHead, EmptyState } from '$lib/components/kit';
	import { Toggle } from '@rokkit/ui';
	import RelayCard from './RelayCard.svelte';
	import { relayInboxState } from './relay-inbox-state.svelte';
	import type { RelayFilter } from './types';
	import * as m from '$lib/paraglide/messages';

	// The inbox list rail (mockup ScrInbox left panel). Pure presentation over
	// relayInboxState: header + filter tabs + a card per shown session, routing
	// selection back through the state. Realtime lives in the state, not here.
	// The filter tabs are the rokkit Toggle (segmented radiogroup) — the design
	// system's own control, not a hand-rolled tab strip.
	const st = relayInboxState;

	const FILTERS = [
		{ label: m.filter_needs(), value: 'needs' },
		{ label: m.filter_running(), value: 'running' },
		{ label: m.filter_finished(), value: 'finished' },
		{ label: m.filter_all(), value: 'all' }
	];
</script>

<!-- Sticky header (section head + filter tabs) never scrolls; the card list below
     is the only scrolling region of the rail. -->
<div class="flex h-full min-h-0 flex-col">
	<div class="shrink-0 px-5 pb-4 pt-6">
		<SectionHead eyebrow={m.inbox_eyebrow()} title={m.inbox_title()} count={st.sessions.length}>
			{#snippet right()}
				{#if st.needsCount > 0}
					<span class="mono text-accent text-xs">{m.inbox_needs_badge({ count: st.needsCount })}</span>
				{/if}
			{/snippet}
		</SectionHead>
		<div class="mt-4">
			<Toggle
				options={FILTERS}
				value={st.filter}
				onchange={(v) => st.setFilter(v as RelayFilter)}
				size="sm"
				label={m.inbox_title()}
			/>
		</div>
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto px-5 pb-6">
		{#if !st.sessions.length}
			<EmptyState kanji="空" title={m.inbox_empty_title()}>{m.inbox_empty_body()}</EmptyState>
		{:else if !st.shown.length}
			<EmptyState kanji="空" title={m.inbox_empty_title()}>{m.inbox_empty_filter()}</EmptyState>
		{:else}
			<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
				{#each st.shown as s (s.id)}
					<RelayCard session={s} selected={s.id === st.selectedId} onopen={(id) => st.select(id)} />
				{/each}
			</div>
			<p class="mono text-ink-faint mt-4 text-xs" style="line-height: 1.5">{m.inbox_sort_hint()}</p>
		{/if}
	</div>
</div>
