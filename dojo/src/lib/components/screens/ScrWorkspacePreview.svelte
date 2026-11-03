<script lang="ts">
	import { dojoLoadNotice } from '$lib/dojo-notices';
	import type { ReadError } from '$lib/read-error';
	import { Screen, Banner, Notice } from '@sensei/ui';
	import { SectionHead, Icon, ClassChip, PhasePill, LadderRung, RuleRow, ConflictCard } from '$lib/components/kit';
	import Spark from '$lib/components/kit/Spark.svelte';
	import type { KitConflict, KitLadderRung, KitWorkspace } from '$lib/components/kit/types';
	import type { WorkspaceMetricsView } from '$lib/workspace-metrics-view';
	import { createWorkspacePreview, type PreviewView } from '$lib/preview/state.svelte';
	import { untrack } from 'svelte';

	// The resolved-constitution drill-in for one workspace (mockup ScrWorkspacePreview)
	// — the "before you start" view a viewer sees when opening a workspace from the
	// list or the landing. A back header, the workspace identity (classification +
	// phase), a classification banner summarizing the resolution, a by-layer /
	// consolidated toggle over the ladder, and (for non-personal work) the
	// conflicts the ladder discarded. Presentational: the page supplies the workspace
	// + the kit ladder / conflicts; the `active` rung + `view` toggle live in the
	// `preview/state` rune store.
	let {
		workspace,
		ladder,
		conflicts,
		error = null,
		metrics = null,
		metricsError = null,
		onBack
	}: {
		workspace: KitWorkspace;
		ladder: KitLadderRung[];
		conflicts: KitConflict[];
		/** A failed constitution read — shown instead of the empty state (#106). */
		error?: ReadError | string | null;
		/** The workspace's per-repository metrics (WP5); null when the page did
		 *  not read them (the personal drill-in) or the read failed. */
		metrics?: WorkspaceMetricsView | null;
		/** A failed metrics read — shown instead of empty charts (#109). */
		metricsError?: string | null;
		onBack?: () => void;
	} = $props();

	// Seed the drill-in state ONCE from the props — opening a different workspace
	// re-mounts this screen (a `/you/workspaces/[id]` navigation), so the resolved
	// ladder + `active`/`view` state is captured at mount. `untrack` makes that
	// initial capture explicit (the same idiom as `ScrYourWork`).
	const preview = untrack(() => createWorkspacePreview(workspace, ladder, conflicts));
	const views: [PreviewView, string][] = [
		['layer', 'By layer'],
		['consolidated', 'Consolidated']
	];
</script>

<Screen>
	{#snippet header()}
		<div class="flex flex-col gap-4 px-4 pt-4 md:px-8 md:pt-8">
			{#if onBack}
				<button
					type="button"
					onclick={onBack}
					class="text-ink-soft inline-flex cursor-pointer items-center gap-1 self-start bg-transparent text-sm"
				>
					<Icon name="alt-arrow-left" size={15} toneClass="text-ink-soft" /> Back to workspaces
				</button>
			{/if}

			<SectionHead kanji="件" eyebrow={'Before you start · ' + workspace.repo} title={workspace.name}>
				{#snippet right()}
					<div class="flex items-center gap-2">
						<ClassChip kind={workspace.classification} />
						<PhasePill phase={workspace.phase} />
					</div>
				{/snippet}
			</SectionHead>
		</div>
	{/snippet}
	<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
		{#if error}
			<Notice size="panel" {...dojoLoadNotice("this workspace's constitution", error)} data-load-error />
		{:else if ladder.length === 0}
			<!-- Honest-empty: the composed constitution is resolved by the daemon in the
			     editor and NOT yet federated to the dōjō (F4). We render the real workspace
			     header above + this truthful state, never a fabricated ladder/conflicts. -->
			<Banner mark="観" tone="normal" title="This workspace's constitution resolves in your editor.">
				sensei composes the ladder — your personal rules layered with this dōjō's rules, most-specific
				wins — locally as you work. The dōjō's view of the resolved constitution is on its way; nothing
				is shown here until it can be shown truthfully.
			</Banner>
		{:else}
			<Banner
				mark="観"
				tone={workspace.classification === 'client' ? 'action' : 'normal'}
				title={workspace.classification === 'client'
					? 'Client work — sources are dereferenced, as they always are.'
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
					<span class="mono text-ink-soft text-xs">
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
									: 'text-ink-soft border border-transparent bg-transparent'}"
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
						<span class="mono text-ink-soft text-xs">what the resolution eliminated, and why</span>
					</div>
					<div class="flex flex-col gap-3">
						{#each conflicts as conflict (conflict.id)}
							<ConflictCard {conflict} />
						{/each}
					</div>
				</div>
			{/if}
		{/if}

		{#if metrics || metricsError}
			<div>
				<div class="flex items-center gap-2 mb-3">
					<Icon name="chart" size={17} toneClass="text-accent" />
					<span class="text-ink text-xs font-semibold uppercase" style="letter-spacing: 0.18em">
						Metrics — per repository
					</span>
					{#if metrics}
						<span class="mono text-ink-soft text-xs">the last {metrics.days} days</span>
					{/if}
				</div>
				{#if metricsError}
					<Notice size="panel" {...dojoLoadNotice("this workspace's metrics", metricsError)} data-load-error />
				{:else if metrics && metrics.repos.length === 0}
					<p class="text-sm text-ink-faint">Nothing sent in this window.</p>
				{:else if metrics}
					<div class="flex flex-col gap-5">
						{#each metrics.repos as repo (repo.repository_id)}
							<div>
								<div class="mono text-ink-soft text-xs mb-2">{repo.name}</div>
								<ul class="grid grid-cols-1 gap-3 md:grid-cols-2 md:gap-4">
									{#each repo.cards as c (c.key)}
										<li class="flex flex-col gap-2 rounded border border-paper-edge p-4">
											<span class="text-xs text-ink-mute">{c.label}</span>
											{#if c.empty}
												<span class="text-sm text-ink-faint">Nothing sent in this window</span>
											{:else}
												<div class="flex items-end justify-between gap-3">
													<span class="text-2xl text-ink">{c.value}</span>
													{#if c.points.length > 1}
														<Spark data={c.points} />
													{/if}
												</div>
											{/if}
										</li>
									{/each}
								</ul>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	</div>
</Screen>
