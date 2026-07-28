<script lang="ts">
	import { onMount } from 'svelte';
	import RelayList from '$lib/relay/RelayList.svelte';
	import RelayDetail from '$lib/relay/RelayDetail.svelte';
	import { relayInboxState } from '$lib/relay/relay-inbox-state.svelte';
	import { loadRelayInbox } from '$lib/relay/relay-inbox';

	// Inbox zone — two-panel master-detail (mockup ScrInbox). Left = RelayList over
	// relayInboxState; right = RelayDetail for the selected run (in-page selection),
	// falling back to the route child (the empty-state page / deep-linked run detail).
	// Selection is state-driven (state.select), so it works on mock data before the
	// real user-wide read lands. Mock-first Load, client-side (a rune singleton must
	// not be populated during SSR). On mount we auto-open the first run so md+ is never
	// blank (mockup). `md+` shows both panels; `<md` shows the rail until a selection.
	let { children } = $props();
	onMount(() => {
		relayInboxState.load(loadRelayInbox());
		relayInboxState.select(relayInboxState.shown[0]?.id ?? null);
	});
	const active = $derived(relayInboxState.selectedId);
</script>

<!-- h-full + overflow-hidden so the two panes scroll independently inside the shell
     (the shell's content area no longer scrolls the whole page). Each column is a
     flex-col with min-h-0 so its child's sticky-header + scroll-body works. -->
<div class="grid h-full grid-cols-1 overflow-hidden md:grid-cols-[minmax(340px,400px)_minmax(0,1fr)]">
	<aside class="border-paper-edge min-h-0 flex-col md:border-r {active ? 'hidden md:flex' : 'flex'}">
		<RelayList />
	</aside>
	<section class="min-h-0 min-w-0 flex-col {active ? 'flex' : 'hidden md:flex'}">
		{#if relayInboxState.selected}
			<RelayDetail />
		{:else}
			{@render children()}
		{/if}
	</section>
</div>
