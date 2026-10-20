<script lang="ts">
	import { Screen } from '@sensei/ui';
	import { SectionHead, EmptyState, Banner, Btn } from '$lib/components/kit';
	import { invalidateAll } from '$app/navigation';
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
<Screen>
	{#snippet header()}
		<div class="px-5 pb-4 pt-6">
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
	{/snippet}
	<div class="px-5 pb-6">
		{#if st.status === 'loading'}
			<div class="space-y-2" data-inbox-loading aria-busy="true" aria-label={m.inbox_loading()}>
				{#each [0, 1, 2] as i (i)}
					<div class="bg-paper-mute h-16 animate-pulse rounded-lg"></div>
				{/each}
			</div>
		{:else if st.status === 'error'}
			<div data-inbox-error>
				<EmptyState kanji="断" title={m.inbox_error_title()}>
					{st.error}
					<div class="mt-3"><Btn onclick={() => invalidateAll()}>{m.inbox_retry()}</Btn></div>
				</EmptyState>
			</div>
		{:else}
			{#if st.unreachable.length || st.plansMissing}
				<div class="mb-3" data-inbox-partial>
					<Banner tone="warning" kanji="欠" title={m.inbox_partial_title()}>
						{#if st.unreachable.length}
							<p>{m.inbox_partial_unreachable({ names: st.unreachable.join(', ') })}</p>
						{/if}
						{#if st.plansMissing}
							<p>{m.inbox_partial_plans({ count: st.plansMissing })}</p>
						{/if}
					</Banner>
				</div>
			{/if}
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
		{/if}
	</div>
</Screen>
