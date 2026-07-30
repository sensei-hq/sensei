<script lang="ts">
	import { onMount } from 'svelte';
	import RelayList from '$lib/relay/RelayList.svelte';
	import RelayDetail from '$lib/relay/RelayDetail.svelte';
	import { relayInboxState } from '$lib/relay/relay-inbox-state.svelte';

	// Inbox zone — two-panel master-detail (mockup ScrInbox). Left = RelayList over
	// relayInboxState; right = RelayDetail for the selected run (in-page selection),
	// falling back to the route child (the empty-state page / deep-linked run detail).
	// The real user-wide sessions are fetched in +layout.ts (fan-out over memberships)
	// and handed to `data.sessions`; we populate the client-side state singleton here in
	// onMount (a rune singleton must not be written during SSR) and auto-open the first
	// run so md+ is never blank. `md+` shows both panels; `<md` shows the rail until a
	// selection.
	let { data, children } = $props();
	onMount(() => {
		relayInboxState.load(data.sessions);
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
