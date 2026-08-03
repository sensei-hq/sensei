<script lang="ts">
	import { SectionHead, Btn, ProjectRow, EmptyState } from '$lib/components/kit';
	import type { KitProject } from '$lib/components/kit/types';

	// The full personal project list (mockup ScrProjects) — a flush card of
	// ProjectRows. Presentational: the page supplies the projects (kit fixtures
	// this chunk). `onOpenProject` bubbles a row open → the preview drill-in.
	// `showDojo` adds the owning-dōjō column (desktop only); `mobile` switches to
	// the stacked phone rows. Degrades to an honest empty state (DJ1).
	let {
		projects = [],
		error = null,
		showDojo = true,
		mobile = false,
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
		mobile?: boolean;
		eyebrow?: string;
		title?: string;
		onOpenProject?: (p: KitProject) => void;
	} = $props();
</script>

<div class="flex flex-col {mobile ? 'p-4 gap-4' : 'p-8 gap-6'}">
	<SectionHead {eyebrow} {title} count={projects.length}>
		{#snippet right()}
			<Btn size="sm" variant="ghost" icon="tuning-2">Filter</Btn>
		{/snippet}
	</SectionHead>

	{#if error}
		<EmptyState kanji="断" title="Couldn't load your projects.">
			{error}. This is a read error, not an empty list — retry in a moment. Nothing has been hidden
			or invented.
		</EmptyState>
	{:else if projects.length}
		<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
			{#each projects as p (p.id)}
				<ProjectRow {p} onopen={onOpenProject} showDojo={showDojo && !mobile} compact={mobile} />
			{/each}
		</div>
	{:else}
		<EmptyState kanji="空" title="No projects yet.">
			Projects appear here once you join a dōjō or sensei starts watching a repository on this
			machine. Nothing is hidden — this is everything you touch.
		</EmptyState>
	{/if}
</div>
