<script lang="ts">
	import { Screen, EmptyState, Notice } from '@sensei/ui';
	import { runUnreadNotice, RUN_NOT_FOUND } from '$lib/dojo-notices';
	import { onMount } from 'svelte';
	import { goto, invalidateAll } from '$app/navigation';
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
	const unread = $derived(runUnreadNotice(data.error, () => invalidateAll(), data.failure));
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
			{#if unread}
				<Notice size="panel" {...unread} />
			{:else}
				<EmptyState {...RUN_NOT_FOUND} />
			{/if}
		</div>
	</Screen>
{/if}
