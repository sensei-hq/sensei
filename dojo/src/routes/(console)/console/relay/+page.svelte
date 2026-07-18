<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import RelayGateCard from '$lib/components/RelayGateCard.svelte';
	import RelayNotifyToggle from '$lib/components/RelayNotifyToggle.svelte';
	import RelayStatusBadge from '$lib/components/RelayStatusBadge.svelte';
	import { relativeAge } from '$lib/triage-view';
	import { progressWidth } from '$lib/relay-view';

	// Relay run list (mockup dojo-relay.jsx "Active"/RelayProjectsBody): every
	// supervised run the caller can see in this tenant, one card each. The card is a
	// link into the run detail route. Above it, a "Needs you" band (mockup "requires
	// you" / RelayProjectsBody.needs) surfaces the pending gates the agent is waiting
	// on, one RelayGateCard each. Presentational only — the load (+page.ts → listRuns
	// + listGates) does the fetching and degrades to empty lists + a surfaced error so
	// the shell still renders; the cards' replies invalidateAll to refresh. Mirrors
	// triage/+page.svelte.
	let { data } = $props();

	// Format an ISO instant as a short local time for the "paused until" line.
	function shortTime(iso: string): string {
		const d = new Date(iso);
		if (Number.isNaN(d.getTime())) return '';
		return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}
</script>

<svelte:head>
	<title>Relay · Dōjō console</title>
</svelte:head>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="継"
		eyebrow="Relay · runs"
		title="Runs you're supervising"
		sub="Every supervised run across your Dōjō — what's running, what's stuck, and what needs you. Approvals and decisions rise to the top; nothing moves without your say."
	>
		{#snippet right()}
			<RelayNotifyToggle tenantKey={data.tenantKey} accessToken={data.accessToken} />
		{/snippet}
	</ConsoleHead>

	<div class="flex-1 overflow-auto" style="padding: 8px 28px 28px">
		{#if data.error}
			<div
				class="bg-warning-soft border-warning-edge text-ink-soft flex items-center gap-2 rounded-xl border text-sm"
				style="padding: 12px 16px; margin-top: 16px"
			>
				<span class="kanji text-warning">検</span>
				<span
					>Live relay is unavailable. <span class="mono text-ink-mute text-xs">{data.error}</span
					></span
				>
			</div>
		{/if}

		{#if data.gates.length > 0}
			<div style="margin-top: 18px">
				<div class="flex items-center gap-2" style="margin-bottom: 12px">
					<span class="kanji text-accent" style="font-size: 13px">要</span>
					<span
						class="text-ink-mute text-xs font-semibold"
						style="letter-spacing: 0.14em; text-transform: uppercase">Needs you</span
					>
					<span class="mono text-accent text-xs">{data.gates.length}</span>
				</div>
				<div class="flex flex-col gap-3">
					{#each data.gates as gate (gate.id)}
						<RelayGateCard
							{gate}
							tenantKey={data.tenantKey}
							accessToken={data.accessToken}
							onReplied={() => invalidateAll()}
						/>
					{/each}
				</div>
			</div>
		{/if}

		{#if data.runs.length === 0 && !data.error}
			<div
				class="border-ink-faint text-ink-mute flex flex-col items-center gap-2 rounded-xl border border-dashed text-center"
				style="padding: 48px 28px; margin-top: 24px"
			>
				<span class="kanji text-ink-faint" style="font-size: 30px">継</span>
				<div class="text-ink-soft text-sm">No runs yet</div>
				<div class="text-ink-mute text-xs">Start one from the daemon and it will appear here.</div>
			</div>
		{/if}

		{#if data.runs.length > 0}
			<div class="flex flex-col gap-3" style="margin-top: 18px">
				{#each data.runs as run (run.id)}
					<a
						href="/console/relay/{run.run_id}"
						class="bg-paper-soft border-paper-edge block rounded-xl border no-underline"
						style="padding: 15px 18px"
					>
						<div class="flex items-start gap-3">
							<span
								class="kanji text-accent flex-shrink-0 text-center"
								style="font-size: 20px; width: 24px; line-height: 1.1">継</span
							>
							<div class="flex-1" style="min-width: 0">
								<div class="flex items-center gap-2">
									<div class="text-ink truncate text-sm font-medium" style="flex: 1; min-width: 0">
										{run.title}
									</div>
									<RelayStatusBadge status={run.status} />
								</div>
								{#if run.goal}
									<div class="text-ink-mute truncate text-xs" style="margin-top: 3px">{run.goal}</div>
								{/if}
							</div>
						</div>

						<div class="flex items-center justify-between" style="margin: 12px 0 5px">
							<span class="mono text-ink-mute text-xs">
								{run.progress_done}/{run.progress_total}
							</span>
							{#if run.current_phase || run.current_feature}
								<span class="text-ink-mute truncate text-xs" style="min-width: 0; text-align: right">
									{[run.current_phase, run.current_feature].filter(Boolean).join(' · ')}
								</span>
							{/if}
						</div>

						<div
							class="bg-paper-mute overflow-hidden rounded-full"
							style="height: 6px"
							role="progressbar"
							aria-valuenow={run.progress_done}
							aria-valuemin="0"
							aria-valuemax={run.progress_total}
						>
							<div
								class="rounded-full {run.status === 'stalled' ? 'bg-warning' : 'bg-ink'}"
								style="height: 100%; width: {progressWidth(run.progress_done, run.progress_total)}"
							></div>
						</div>

						<div class="flex items-center justify-between" style="margin-top: 10px">
							{#if run.status === 'paused' && run.paused_until}
								<span class="text-ink-soft text-xs">
									Paused until {shortTime(run.paused_until)}{run.pause_reason
										? ` · ${run.pause_reason}`
										: ''}
								</span>
							{:else if run.last_event_at}
								<span class="text-ink-mute text-xs">
									Last progress {relativeAge(run.last_event_at)}
								</span>
							{:else}
								<span class="text-ink-faint text-xs">No progress yet</span>
							{/if}
							<span class="text-ink-faint text-sm">→</span>
						</div>
					</a>
				{/each}
			</div>
		{/if}
	</div>
</div>
