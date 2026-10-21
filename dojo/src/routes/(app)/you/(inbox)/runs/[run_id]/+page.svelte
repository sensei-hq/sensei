<script lang="ts">
	import { Screen } from '@sensei/ui';
	import { onMount } from 'svelte';
	import { goto, invalidateAll } from '$app/navigation';
	import { EmptyState } from '$lib/components/kit';
	import DojoJoinEmpty from '$lib/components/DojoJoinEmpty.svelte';
	import RelayDetail from '$lib/relay/RelayDetail.svelte';
	import { toRelaySession } from '$lib/relay/relay-inbox-map';
	import type { SendReply } from '$lib/relay/relay-inbox-state.svelte';
	import { replyToGate } from '$lib/relay-data';
	import { youHref } from '$lib/nav';
	import { subscribeRelay } from '$lib/relay-realtime';
	import { env } from '$env/dynamic/public';

	// One run's detail, opened by a /you/runs/<id> link when the inbox does not hold
	// it — the same RelayDetail the inbox shows (#293). The load (+page.ts) fetches
	// the run, its segments and its pending gates; this builds the session and keeps
	// it live over realtime.
	let { data } = $props();

	const session = $derived(
		data.run && data.tenantKey
			? toRelaySession(data.run, data.gates, data.segments, data.tenantKey)
			: null
	);
	const send: SendReply = (tenantKey, inboxId, reply) =>
		replyToGate(tenantKey, inboxId, reply, { fetch, accessToken: data.accessToken ?? null });

	onMount(() =>
		// Refresh this run live: a raised gate, a segment or progress update.
		subscribeRelay({
			url: env.PUBLIC_SUPABASE_URL,
			anonKey: env.PUBLIC_SUPABASE_ANON_KEY,
			accessToken: data.accessToken,
			topic: `relay:run:${data.runId}`,
			onChange: () => invalidateAll()
		})
	);
</script>

<svelte:head><title>{data.run?.title ?? 'Run'} · Dōjō</title></svelte:head>

{#if data.noMembership}
	<Screen>
		<DojoJoinEmpty what="relay runs" />
	</Screen>
{:else if session}
	<RelayDetail
		{session}
		{send}
		notice={data.error}
		onBack={() => goto(youHref())}
		onAnswered={() => invalidateAll()}
	/>
{:else}
	<Screen>
		<div class="p-6">
			<EmptyState kanji="観" title={data.error ? 'Live relay is unavailable.' : 'This run is not in your dōjōs.'}>
				{data.error ?? 'It may have finished long ago, or belong to a dōjō you are not in.'}
			</EmptyState>
		</div>
	</Screen>
{/if}
