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
		showDojo = true,
		mobile = false,
		eyebrow = 'You · every project you touch',
		title = 'Projects',
		onOpenProject
	}: {
		projects?: KitProject[];
		showDojo?: boolean;
		mobile?: boolean;
		eyebrow?: string;
		title?: string;
		onOpenProject?: (p: KitProject) => void;
	} = $props();
</script>

<div class="flex flex-col" style="padding: {mobile ? '16px' : '32px'}; gap: {mobile ? '16px' : '24px'}">
	<SectionHead {eyebrow} {title} count={projects.length}>
		{#snippet right()}
			<Btn size="sm" variant="ghost" icon="tuning-2">Filter</Btn>
		{/snippet}
	</SectionHead>

	{#if projects.length}
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
