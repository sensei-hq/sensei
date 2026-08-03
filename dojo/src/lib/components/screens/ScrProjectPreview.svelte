<script lang="ts">
	import {
		SectionHead,
		Banner,
		Icon,
		ClassChip,
		PhasePill,
		LadderRung,
		RuleRow,
		ConflictCard
	} from '$lib/components/kit';
	import type { KitConflict, KitLadderRung, KitProject } from '$lib/components/kit/types';
	import { createProjectPreview, type PreviewView } from '$lib/preview/state.svelte';
	import { untrack } from 'svelte';

	// The resolved-constitution drill-in for one project (mockup ScrProjectPreview)
	// — the "before you start" view a viewer sees when opening a project from the
	// list or the landing. A back header, the project identity (classification +
	// phase), a classification banner summarizing the resolution, a by-layer /
	// consolidated toggle over the ladder, and (for non-personal work) the
	// conflicts the ladder discarded. Presentational: the page supplies the project
	// + the kit ladder / conflicts; the `active` rung + `view` toggle live in the
	// `preview/state` rune store.
	let {
		project,
		ladder,
		conflicts,
		onBack
	}: {
		project: KitProject;
		ladder: KitLadderRung[];
		conflicts: KitConflict[];
		onBack?: () => void;
	} = $props();

	// Seed the drill-in state ONCE from the props — opening a different project
	// re-mounts this screen (a `/you/projects/[id]` navigation), so the resolved
	// ladder + `active`/`view` state is captured at mount. `untrack` makes that
	// initial capture explicit (the same idiom as `ScrYourWork`).
	const preview = untrack(() => createProjectPreview(project, ladder, conflicts));
	const views: [PreviewView, string][] = [
		['layer', 'By layer'],
		['consolidated', 'Consolidated']
	];
</script>

<div class="flex flex-col p-8 gap-6">
	{#if onBack}
		<button
			type="button"
			onclick={onBack}
			class="text-ink-mute inline-flex cursor-pointer items-center gap-1 self-start bg-transparent text-sm"
		>
			<Icon name="alt-arrow-left" size={15} toneClass="text-ink-mute" /> Back to projects
		</button>
	{/if}

	<SectionHead kanji="件" eyebrow={'Before you start · ' + project.repo} title={project.name}>
		{#snippet right()}
			<div class="flex items-center gap-2">
				<ClassChip kind={project.classification} />
				<PhasePill phase={project.phase} />
			</div>
		{/snippet}
	</SectionHead>

	{#if ladder.length === 0}
		<!-- Honest-empty: the composed constitution is resolved by the daemon in the
		     editor and NOT yet federated to the dōjō (F4). We render the real project
		     header above + this truthful state, never a fabricated ladder/conflicts. -->
		<Banner kanji="観" tone="neutral" title="This project's constitution resolves in your editor.">
			sensei composes the ladder — your personal rules layered with this dōjō's rules, most-specific
			wins — locally as you work. The dōjō's view of the resolved constitution is on its way; nothing
			is shown here until it can be shown truthfully.
		</Banner>
	{:else}
		<Banner
			kanji="観"
			tone={project.classification === 'client' ? 'accent' : 'neutral'}
			title={project.classification === 'client'
				? 'Client engagement — sources are dereferenced, as they always are.'
				: 'These rules resolve here, most-specific wins.'}
		>
			{preview.rungs.length} scopes compose this constitution · {preview.effective.length} rules apply
			· {preview.locks} locked (★){preview.discarded.length
				? ' · ' + preview.discarded.length + ' discarded by the ladder'
				: ''}.
		</Banner>

		<div>
			<div class="flex items-center gap-2 mb-3">
				<Icon name="layers-minimalistic" size={17} toneClass="text-accent" />
				<span class="text-ink text-xs font-semibold uppercase" style="letter-spacing: 0.18em">
					{preview.view === 'layer' ? 'The ladder — broad → specific' : 'Consolidated constitution'}
				</span>
				<span class="mono text-ink-mute text-xs">
					{preview.view === 'layer'
						? 'tap a rung to focus'
						: preview.effective.length + ' effective rules'}
				</span>
				<span class="flex-1"></span>
				<div class="bg-paper-mute flex rounded" style="gap: 2px; padding: 2px">
					{#each views as [id, label] (id)}
						<button
							type="button"
							onclick={() => preview.setView(id)}
							class="rounded text-xs {preview.view === id
								? 'bg-paper text-ink border-paper-edge border'
								: 'text-ink-mute border border-transparent bg-transparent'}"
							style="padding: 4px 10px"
						>
							{label}
						</button>
					{/each}
				</div>
			</div>

			{#if preview.view === 'layer'}
				<div class="flex flex-col gap-3">
					{#each preview.rungs as rung (rung.id)}
						<LadderRung {rung} active={preview.active} onSelect={preview.setActive} />
					{/each}
				</div>
			{:else}
				<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
					{#each preview.effective as rule, i (i)}
						<RuleRow {rule} showLevel onJump={(r) => preview.jumpTo(r.level ?? '')} />
					{/each}
				</div>
			{/if}
		</div>

		{#if preview.showConflicts}
			<div>
				<div class="flex items-center gap-2 mb-3">
					<Icon name="danger-triangle" size={17} toneClass="text-warning" />
					<span class="text-ink text-xs font-semibold uppercase" style="letter-spacing: 0.18em">
						Discarded by the ladder
					</span>
					<span class="mono text-ink-mute text-xs">what the resolution eliminated, and why</span>
				</div>
				<div class="flex flex-col gap-3">
					{#each conflicts as conflict (conflict.id)}
						<ConflictCard {conflict} />
					{/each}
				</div>
			</div>
		{/if}
	{/if}
</div>
