<script lang="ts">
	import type { RelayGate } from '$lib/relay-data';
	import { orderGatesByUrgency, blockedSummary, gateSeverity, gateHref } from '$lib/relay-view';
	import { relativeAge } from '$lib/triage-view';
	import DojoChip from './DojoChip.svelte';

	// Relay "blocked on you" home (P4.6): the away-from-keyboard landing that
	// aggregates every pending gate across the caller's runs and surfaces what's
	// waiting on the human, most-urgent first, each row linking to its run's gate
	// card. Presentational only — the ordering / counts / deep-link all come from the
	// pure relay-view helpers, never reimplemented here. Refresh-based for now: the
	// page's existing invalidateAll() on reply re-fetches this section (no realtime
	// yet); P4.2 will make gate removal live. No fetching/mutations/stores here.
	let { gates }: { gates: RelayGate[] } = $props();

	const ordered = $derived(orderGatesByUrgency(gates));
	const summary = $derived(blockedSummary(gates));

	// The stripped prompt for a gate, or a friendly fallback — matches RelayGateCard.
	function ask(gate: RelayGate): string {
		return typeof gate.payload.prompt === 'string' && gate.payload.prompt.trim()
			? gate.payload.prompt
			: 'The run needs a decision';
	}
</script>

{#snippet rowBody(gate: RelayGate)}
	{@const sev = gateSeverity(gate)}
	<div class="flex items-start gap-3">
		<span
			class="kanji text-accent flex-shrink-0 text-center text-xl" style="width: 24px; line-height: 1.1">要</span
		>
		<div class="flex-1" style="min-width: 0">
			<div class="text-ink text-sm font-medium" style="line-height: 1.5">{ask(gate)}</div>
			<div class="flex items-center gap-2 flex-wrap" style="margin-top: 4px">
				<span class="text-ink-mute truncate text-xs">{gate.run_title ?? 'View run'}</span>
				<span class="text-ink-faint text-xs">·</span>
				<span class="text-ink-faint text-xs">{relativeAge(gate.created_at)}</span>
				<DojoChip toneClass={sev === 'blocking' ? 'text-accent' : 'text-ink-mute'}>
					{sev === 'blocking' ? 'Needs you' : 'Heads-up'}
				</DojoChip>
			</div>
		</div>
		{#if gateHref(gate)}
			<span class="text-ink-faint flex-shrink-0 text-sm">→</span>
		{/if}
	</div>
{/snippet}

{#if gates.length === 0}
	<div
		class="border-ink-faint text-ink-mute flex flex-col items-center gap-2 rounded-xl border border-dashed text-center"
		style="padding: 48px 28px; margin-top: 24px"
	>
		<span class="kanji text-ink-faint text-2xl">静</span>
		<div class="text-ink-soft text-sm">Nothing's waiting on you</div>
		<div class="text-ink-mute text-xs">
			the runs are progressing — sensei will surface anything that needs your call.
		</div>
	</div>
{:else}
	<div style="margin-top: 18px">
		<div class="flex items-center gap-2" style="margin-bottom: 12px">
			<span class="kanji text-accent text-sm">要</span>
			<span
				class="text-ink-mute text-xs font-semibold"
				style="letter-spacing: 0.14em; text-transform: uppercase">Blocked on you</span
			>
			<span class="mono text-accent text-xs">{summary.total}</span>
			{#if summary.blocking > 0}
				<span class="text-ink-faint text-xs">{summary.blocking} blocking</span>
			{/if}
		</div>

		<div class="flex flex-col gap-3">
			{#each ordered as gate (gate.id)}
				{@const href = gateHref(gate)}
				{#if href}
					<a
						{href}
						class="bg-paper-soft border-paper-edge block rounded-xl border no-underline"
						style="padding: 15px 18px"
					>
						{@render rowBody(gate)}
					</a>
				{:else}
					<div
						class="bg-paper-soft border-paper-edge rounded-xl border"
						style="padding: 15px 18px"
					>
						{@render rowBody(gate)}
					</div>
				{/if}
			{/each}
		</div>
	</div>
{/if}
