<script lang="ts">
	import { Screen } from '@sensei/ui';
	import { SectionHead, Btn, WorkspaceRow, EmptyState } from '$lib/components/kit';
	import type { KitWorkspace } from '$lib/components/kit/types';

	// The full personal workspace list (mockup ScrWorkspaces) — a flush card of
	// WorkspaceRows. Presentational: the page supplies the workspaces (kit fixtures
	// this chunk). `onOpenWorkspace` bubbles a row open → the preview drill-in.
	// `showDojo` adds the owning-dōjō column, which WorkspaceRow itself hides below
	// `md`. Degrades to an honest empty state (DJ1).
	let {
		workspaces = [],
		error = null,
		showDojo = true,
		eyebrow = 'You · every workspace you touch',
		title = 'Workspaces',
		onOpenWorkspace
	}: {
		workspaces?: KitWorkspace[];
		/** A read-failure message — when set, the screen shows an error state
		 *  instead of the honest-empty one, so a failure is never masked as "no
		 *  workspaces" (F1 honesty: error ≠ empty). */
		error?: string | null;
		showDojo?: boolean;
		eyebrow?: string;
		title?: string;
		onOpenWorkspace?: (p: KitWorkspace) => void;
	} = $props();
</script>

<Screen>
	{#snippet header()}
		<div class="px-4 pt-4 md:px-8 md:pt-8">
			<SectionHead {eyebrow} {title} count={workspaces.length}>
				{#snippet right()}
					<Btn size="sm" variant="ghost" icon="tuning-2">Filter</Btn>
				{/snippet}
			</SectionHead>
		</div>
	{/snippet}
	<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
		{#if error}
			<EmptyState kanji="断" title="Couldn't load your workspaces.">
				{error}. This is a read error, not an empty list — retry in a moment. Nothing has been hidden
				or invented.
			</EmptyState>
		{:else if workspaces.length}
			<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
				{#each workspaces as p (p.id)}
					<WorkspaceRow {p} onopen={onOpenWorkspace} {showDojo} />
				{/each}
			</div>
		{:else}
			<EmptyState kanji="空" title="No workspaces yet.">
				Workspaces appear here once you join a dōjō or sensei starts watching a repository on this
				machine. Nothing is hidden — this is everything you touch.
			</EmptyState>
		{/if}
	</div>
</Screen>
