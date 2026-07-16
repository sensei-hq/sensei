<script lang="ts">
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import { relativeAge } from '$lib/triage-view';
	import { progressWidth, statusBadge } from '$lib/relay-view';

	// Relay run list (mockup dojo-relay.jsx "Active"/RelayProjectsBody): every
	// supervised run the caller can see in this tenant, one card each. The card is a
	// link into the run detail route (a later chunk). Presentational only — the load
	// (+page.ts → relay-data.listRuns) does the fetching and degrades to an empty
	// list + surfaced error so the shell still renders. Mirrors triage/+page.svelte.
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
	/>

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
					{@const badge = statusBadge(run.status)}
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
									<DojoChip toneClass={badge.toneClass}>{badge.label}</DojoChip>
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
