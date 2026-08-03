<script lang="ts">
	import {
		SectionHead,
		StatBadge,
		NeedsYouBand,
		ListSection,
		Btn,
		ProjectRow,
		EmptyState
	} from '$lib/components/kit';
	import type { KitProject, KitNeed, KitOrgStats } from '$lib/components/kit/types';
	import { type NeedsAction } from '$lib/components/kit/needs';

	// The org-zone home (mockup ScrOrgHome) — the projects in the dōjō's
	// jurisdiction plus an org "needs a maintainer" band and the jurisdiction stat
	// row. Renders at /org/[slug]. The stat badges (members · needs · projects)
	// summarize the jurisdiction (desktop only — the phone drops the row); the band
	// is the maintainer's remote control (act inline, nothing routes away); the
	// list is every project under this dōjō. Presentational: the page supplies the
	// projects · needs · stats (kit fixtures this chunk); `onOpenProject` bubbles a
	// row open → the org project preview, `onAct` an inline band action. Degrades to
	// an honest empty state when nothing is in jurisdiction (DJ1).
	let {
		orgName,
		projects = [],
		needs = [],
		stats,
		resolved,
		mobile = false,
		onOpenProject,
		onAct
	}: {
		orgName: string;
		projects?: KitProject[];
		needs?: KitNeed[];
		stats?: KitOrgStats;
		resolved?: Record<string, string>;
		mobile?: boolean;
		onOpenProject?: (p: KitProject) => void;
		onAct?: (item: KitNeed, action: NeedsAction) => void;
	} = $props();
</script>

<div class="flex flex-col {mobile ? 'p-4 gap-4' : 'p-8 gap-6'}">
	<SectionHead
		eyebrow={orgName + ' · jurisdiction'}
		title={projects.length + ' projects under this dōjō'}
	/>

	{#if !mobile && stats}
		<div
			class="bg-paper-soft border-paper-edge flex items-center rounded-lg border"
			style="padding: 16px 32px; gap: 48px"
		>
			{#if stats.members != null}
				<StatBadge n={stats.members} label="members" />
				<span class="bg-paper-edge flex-shrink-0" style="width: 1px; height: 34px"></span>
			{/if}
			{#if stats.needs != null}
				<StatBadge
					n={stats.needs}
					label="need a maintainer"
					sub="across this jurisdiction"
					toneClass="text-accent"
				/>
				<span class="bg-paper-edge flex-shrink-0" style="width: 1px; height: 34px"></span>
			{/if}
			<StatBadge n={stats.projects} label="projects" sub="in flight" />
		</div>
	{/if}

	<NeedsYouBand
		items={needs}
		{resolved}
		title="Needs a maintainer"
		{mobile}
		{onAct}
	/>

	<ListSection icon="folder" title="Projects" count={projects.length}>
		{#snippet right()}
			<Btn size="sm" variant="ghost" icon="tuning-2">By team</Btn>
		{/snippet}
		{#if projects.length}
			{#each projects as p (p.id)}
				<ProjectRow {p} onopen={onOpenProject} showDojo={false} compact={mobile} />
			{/each}
		{:else}
			<EmptyState kanji="空" title="No projects in this jurisdiction yet.">
				Projects appear here once sensei starts watching a repository under this dōjō. Nothing is
				hidden — this is everything the dōjō governs.
			</EmptyState>
		{/if}
	</ListSection>
</div>
