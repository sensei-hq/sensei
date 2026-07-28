<script lang="ts">
	import { onMount } from 'svelte';
	import RelayList from '$lib/relay/RelayList.svelte';
	import { relayInboxState } from '$lib/relay/relay-inbox-state.svelte';
	import { loadRelayInbox } from '$lib/relay/relay-inbox';

	// Inbox zone — two-panel master-detail (mockup ScrInbox). Left = RelayList over
	// relayInboxState; right = the detail. Selection is state-driven (state.select),
	// so it works on mock data before the real user-wide read lands. Mock-first Load,
	// client-side (a rune singleton must not be populated during SSR). `md+` shows
	// both panels; `<md` shows the rail until a session is selected.
	let { children } = $props();
	onMount(() => relayInboxState.load(loadRelayInbox()));
	const active = $derived(relayInboxState.selectedId);
</script>

<div class="grid min-h-full grid-cols-1 md:grid-cols-[minmax(340px,400px)_minmax(0,1fr)]">
	<aside class="border-paper-edge md:border-r {active ? 'hidden md:block' : 'block'}">
		<RelayList />
	</aside>
	<section class="min-w-0 {active ? 'block' : 'hidden md:block'}">
		{@render children()}
	</section>
</div>
