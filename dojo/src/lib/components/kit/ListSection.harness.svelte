<script lang="ts">
	import ListSection from './ListSection.svelte';
	import ProjectRow from './ProjectRow.svelte';
	import { projects } from './fixtures';

	// Test-friendly wrapper for ListSection: a header + a couple of fixture rows in
	// the flush card, exercising the "header + rows" recipe the way a screen uses it.
	let {
		title = 'Active projects',
		icon = 'folder',
		count = 3
	}: { title?: string; icon?: string; count?: number } = $props();
</script>

<ListSection {title} {icon} {count}>
	{#each projects as p (p.id)}
		<ProjectRow {p} />
	{/each}
</ListSection>
