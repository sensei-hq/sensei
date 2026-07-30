<script lang="ts">
	import { EmptyState } from '$lib/components/kit';

	// The /you index is the RIGHT panel of the two-panel inbox when no run is open —
	// the list rail lives in +layout.svelte, which auto-opens the first run on md+.
	// This placeholder shows when there's nothing to auto-open (an empty inbox) or
	// briefly before the redirect settles.
</script>

<svelte:head><title>Inbox · Dōjō</title></svelte:head>

<div class="flex h-full items-center justify-center p-8">
	<EmptyState kanji="継" title="Select a session">
		Pick a run on the left to see its plan, activity, and anything that needs you.
	</EmptyState>
</div>
