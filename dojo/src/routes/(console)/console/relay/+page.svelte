<script lang="ts">
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import RelayBlockedHome from '$lib/components/RelayBlockedHome.svelte';
	import RelayNotifyToggle from '$lib/components/RelayNotifyToggle.svelte';
	import RelayStatusBadge from '$lib/components/RelayStatusBadge.svelte';
	import { relativeAge } from '$lib/triage-view';
	import { progressWidth } from '$lib/relay-view';
	import { onMount } from 'svelte';
	import { invalidateAll } from '$app/navigation';
	import { env } from '$env/dynamic/public';
	import { subscribeRelay } from '$lib/relay-realtime';

	// Relay run list (mockup dojo-relay.jsx "Active"/RelayProjectsBody): every
	// supervised run the caller can see in this tenant, one card each. The card is a
	// link into the run detail route. Above it, the P4.6 "Blocked on you" home
	// (RelayBlockedHome) is the first-class away-from-keyboard landing — it aggregates
	// the pending gates the agent is waiting on across all runs, urgency-ordered, each
	// linking to its gate card. Presentational only — the load (+page.ts → listRuns +
	// listGates) does the fetching and degrades to empty lists + a surfaced error so
	// the shell still renders. Refresh-based for now: gate replies happen on the run
	// detail page and invalidateAll to refresh this section (P4.2 will make removal
	// live via realtime). Mirrors triage/+page.svelte.
	let { data } = $props();

	// P4.2 realtime swap: subscribe to Supabase Realtime on the signed-in user's
	// relay rows so a raised gate / status change refreshes this list LIVE, not just
	// on a user action. subscribeRelay authorizes as the user (data.accessToken →
	// the session JWT), so RLS scopes the stream to their own runs; onChange debounces
	// an invalidateAll so a burst of changes triggers a single refresh. It's a no-op
	// under SSR or when unauthenticated (the action-refresh path still works), and its
	// teardown removes the channel on unmount so navigating away never leaks a channel.
	onMount(() =>
		subscribeRelay({
			url: env.PUBLIC_SUPABASE_URL,
			anonKey: env.PUBLIC_SUPABASE_ANON_KEY,
			accessToken: data.accessToken,
			onChange: () => invalidateAll()
		})
	);

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

	<div class="flex-1 overflow-auto px-4 pt-2 pb-7 md:px-7">
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

		{#if !data.error}
			<!-- P4.6 "Blocked on you" home — the first-class landing across all runs.
				 Owns its own empty state ("nothing's waiting on you"), so it renders
				 whether or not there are gates; hidden only when the shell is erroring. -->
			<RelayBlockedHome gates={data.gates} />
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
