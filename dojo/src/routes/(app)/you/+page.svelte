<script lang="ts">
	import { goto } from '$app/navigation';
	import ScrInbox from '$lib/components/screens/ScrInbox.svelte';
	import { youHref } from '$lib/nav';
	import type { KitRun } from '$lib/components/kit/types';

	// The personal landing is the Inbox — every in-flight session in one list.
	// Opening a row routes to the session detail (/you/runs/[run_id]); approve /
	// decide / chat are answered there, not as separate surfaces.
	let { data } = $props();

	function openRun(r: KitRun) {
		goto(youHref('runs') + '/' + r.id);
	}
</script>

<svelte:head><title>Inbox · Dōjō</title></svelte:head>

<ScrInbox inbox={data.inbox} error={data.error} onOpen={openRun} />
