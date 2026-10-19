<script lang="ts">
	import RelayList from '$lib/relay/RelayList.svelte';
	import RelayDetail from '$lib/relay/RelayDetail.svelte';
	import { relayInboxState } from '$lib/relay/relay-inbox-state.svelte';

	// Inbox zone — two-panel master-detail (mockup ScrInbox). Left = RelayList over
	// relayInboxState; right = RelayDetail for the selected run (in-page selection),
	// falling back to the route child (the empty-state page / deep-linked run detail).
	// The real user-wide sessions stream in from +layout.ts (fan-out over memberships)
	// as `data.inbox`. An $effect — which never runs during SSR, so the rune singleton
	// is not written there — marks the load begun, then settles it and auto-opens the
	// first run so md+ is never blank. It re-runs when `invalidateAll()` refetches,
	// which is the error state's retry. `md+` shows both panels; `<md` shows the rail
	// until a selection.
	let { data, children } = $props();
	$effect(() => {
		const pending = data.inbox;
		let stale = false;
		relayInboxState.begin();
		pending.then((outcome) => {
			if (stale) return;
			if (outcome.ok) relayInboxState.loaded(outcome.result);
			else relayInboxState.fail(outcome.message);
			relayInboxState.select(relayInboxState.shown[0]?.id ?? null);
		});
		return () => {
			stale = true;
		};
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
