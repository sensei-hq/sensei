<script lang="ts">
	import EmptyState from './EmptyState.svelte';

	// Test-friendly wrapper for EmptyState: an optional body child; when no title
	// is passed the component's default voice copy ("Still listening.") shows.
	let { title, body }: { title?: string; body?: string } = $props();
</script>

{#if body}
	<EmptyState {title}>{body}</EmptyState>
{:else}
	<EmptyState {title} />
{/if}
