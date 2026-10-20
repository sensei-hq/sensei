<script lang="ts">
	import { Screen } from '@sensei/ui';
	import { SectionHead, Btn, ProjectRow, EmptyState } from '$lib/components/kit';
	import type { KitProject } from '$lib/components/kit/types';

	// The full personal project list (mockup ScrProjects) — a flush card of
	// ProjectRows. Presentational: the page supplies the projects (kit fixtures
	// this chunk). `onOpenProject` bubbles a row open → the preview drill-in.
	// `showDojo` adds the owning-dōjō column, which ProjectRow itself hides below
	// `md`. Degrades to an honest empty state (DJ1).
	let {
		projects = [],
		error = null,
		showDojo = true,
		eyebrow = 'You · every project you touch',
		title = 'Projects',
		onOpenProject
	}: {
		projects?: KitProject[];
		/** A read-failure message — when set, the screen shows an error state
		 *  instead of the honest-empty one, so a failure is never masked as "no
		 *  projects" (F1 honesty: error ≠ empty). */
		error?: string | null;
		showDojo?: boolean;
		eyebrow?: string;
		title?: string;
		onOpenProject?: (p: KitProject) => void;
	} = $props();
</script>

<Screen>
	{#snippet header()}
		<div class="px-4 pt-4 md:px-8 md:pt-8">
			<SectionHead {eyebrow} {title} count={projects.length}>
				{#snippet right()}
					<Btn size="sm" variant="ghost" icon="tuning-2">Filter</Btn>
				{/snippet}
			</SectionHead>
		</div>
	{/snippet}
	<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
		{#if error}
			<EmptyState kanji="断" title="Couldn't load your projects.">
				{error}. This is a read error, not an empty list — retry in a moment. Nothing has been hidden
				or invented.
			</EmptyState>
		{:else if projects.length}
			<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
				{#each projects as p (p.id)}
					<ProjectRow {p} onopen={onOpenProject} {showDojo} />
				{/each}
			</div>
		{:else}
			<EmptyState kanji="空" title="No projects yet.">
				Projects appear here once you join a dōjō or sensei starts watching a repository on this
				machine. Nothing is hidden — this is everything you touch.
			</EmptyState>
		{/if}
	</div>
</Screen>
