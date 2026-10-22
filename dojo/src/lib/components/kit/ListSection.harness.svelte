<script lang="ts">
	import ListSection from './ListSection.svelte';
	import WorkspaceRow from './WorkspaceRow.svelte';
	import { workspaces } from './fixtures';

	// Test-friendly wrapper for ListSection: a header + a couple of fixture rows in
	// the flush card, exercising the "header + rows" recipe the way a screen uses it.
	let {
		title = 'Active workspaces',
		icon = 'folder',
		count = 3
	}: { title?: string; icon?: string; count?: number } = $props();
</script>

<ListSection {title} {icon} {count}>
	{#each workspaces as p (p.id)}
		<WorkspaceRow {p} />
	{/each}
</ListSection>
