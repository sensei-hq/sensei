<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import ScrInbox from '$lib/components/screens/ScrInbox.svelte';
	import { youHref } from '$lib/nav';
	import type { KitRun } from '$lib/components/kit/types';

	// The Inbox zone is a two-panel master-detail (mockup ScrInbox): a persistent
	// list rail on the left + the selected run's detail on the right. The rail data
	// loads once in +layout.ts so it survives navigation between runs; selecting a
	// row navigates to /you/runs/[id] — a self-contained route rendered into the
	// slot — which on md+ just swaps the right panel while the rail stays put. On
	// <md only one pane shows at a time: the list at /you, the detail on a run.
	let { data, children } = $props();

	const activeRunId = $derived(page.params.run_id ?? null);
	function openRun(r: KitRun) {
		goto(youHref('runs') + '/' + r.id);
	}

	// Desktop master-detail: when no run is open, auto-open the first in-flight
	// session so the detail panel is never blank on landing (matches the mockup's
	// first-row selection). Runs from the layout (which always has the rail data),
	// guarded to md+ so <md keeps the list-only landing. The guard flips off once a
	// run is active, so it never loops.
	$effect(() => {
		const first = data.inbox?.[0]?.run?.id;
		if (!activeRunId && first && window.matchMedia('(min-width: 768px)').matches) {
			goto(youHref('runs') + '/' + first, { replaceState: true });
		}
	});
</script>

<div class="grid min-h-full grid-cols-1 md:grid-cols-[minmax(340px,400px)_minmax(0,1fr)]">
	<aside class="border-paper-edge md:border-r {activeRunId ? 'hidden md:block' : 'block'}">
		<ScrInbox inbox={data.inbox} error={data.error} selectedId={activeRunId} onOpen={openRun} />
	</aside>
	<section class="min-w-0 {activeRunId ? 'block' : 'hidden md:block'}">
		{@render children()}
	</section>
</div>
