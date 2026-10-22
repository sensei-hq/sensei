<script lang="ts">
	import { Screen } from '@sensei/ui';
	import {
		SectionHead,
		StatBadge,
		NeedsYouBand,
		ListSection,
		Btn,
		WorkspaceRow,
		EmptyState
	} from '$lib/components/kit';
	import type { KitWorkspace, KitNeed, KitOrgStats } from '$lib/components/kit/types';
	import { type NeedsAction } from '$lib/components/kit/needs';

	// The org-zone home (mockup ScrOrgHome) — the workspaces in the dōjō's
	// jurisdiction plus an org "needs a maintainer" band and the jurisdiction stat
	// row. Renders at /org/[slug]. The stat badges (members · needs · workspaces)
	// summarize the jurisdiction (desktop only — the phone drops the row); the band
	// is the maintainer's remote control (act inline, nothing routes away); the
	// list is every workspace under this dōjō. Presentational: the page supplies the
	// workspaces · needs (the open triage queue) · stats; `onOpenWorkspace` bubbles a
	// row open → the org workspace preview, `onAct` an inline band action. Degrades to
	// an honest empty state when nothing is in jurisdiction (DJ1).
	let {
		orgName,
		workspaces = [],
		needs = [],
		needsError = null,
		stats,
		resolved,
		onOpenWorkspace,
		onAct
	}: {
		orgName: string;
		workspaces?: KitWorkspace[];
		needs?: KitNeed[];
		/** A failed read of the band's queue — shown, never an empty band. */
		needsError?: string | null;
		stats?: KitOrgStats;
		resolved?: Record<string, string>;
		onOpenWorkspace?: (p: KitWorkspace) => void;
		onAct?: (item: KitNeed, action: NeedsAction) => void;
	} = $props();
</script>

<Screen>
	{#snippet header()}
		<div class="px-4 pt-4 md:px-8 md:pt-8">
			<SectionHead
				eyebrow={orgName + ' · jurisdiction'}
				title={workspaces.length + ' workspaces under this dōjō'}
			/>
		</div>
	{/snippet}
	<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
		{#if stats}
			<div
				class="bg-paper-soft border-paper-edge hidden items-center rounded-lg border md:flex"
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
				<StatBadge n={stats.workspaces} label="workspaces" sub="in flight" />
			</div>
		{/if}

		{#if needsError}
			<p class="text-danger m-0 text-xs" data-needs-error>Needs a maintainer — {needsError}</p>
		{:else}
			<NeedsYouBand items={needs} {resolved} title="Needs a maintainer" {onAct} />
		{/if}

		<ListSection icon="folder" title="Workspaces" count={workspaces.length}>
			{#snippet right()}
				<Btn size="sm" variant="ghost" icon="tuning-2">By team</Btn>
			{/snippet}
			{#if workspaces.length}
				{#each workspaces as p (p.id)}
					<WorkspaceRow {p} onopen={onOpenWorkspace} showDojo={false} />
				{/each}
			{:else}
				<EmptyState kanji="空" title="No workspaces in this jurisdiction yet.">
					Workspaces appear here once sensei starts watching a repository under this dōjō. Nothing is
					hidden — this is everything the dōjō governs.
				</EmptyState>
			{/if}
		</ListSection>
	</div>
</Screen>
