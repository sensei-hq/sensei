<script lang="ts">
	import { Screen } from '@sensei/ui';
	import { SectionHead, Icon, LoadError } from '$lib/components/kit';
	import type { SdlcFigure } from '@sensei/sdlc-figures';
	import type { DeliveryPageState } from '$lib/delivery-state.svelte';
	import { TEAM_EXPLAINER, type DeliveryBoardView, type TeamView } from '$lib/workspace-delivery-view';

	// A workspace's delivery (spec tracker-delivery-metrics D5; #352, #353): each
	// tracker board's predictability and its sprints' seven figures, as the boards'
	// kept copies report them, with the team's merged effort beside them, and a
	// selected sprint's tickets by title, each opening to its team figures.
	// Presentational: the page supplies the view, the read error and the selection.
	let {
		workspaceName,
		view,
		error,
		state,
		onBack
	}: {
		workspaceName: string;
		view: { boards: DeliveryBoardView[] } | null;
		/** A failed read — shown instead of an empty page. */
		error: string | null;
		state: DeliveryPageState;
		onBack?: () => void;
	} = $props();

	const th = 'px-2 pb-1 text-left text-xs font-normal text-ink-soft';
	const td = 'px-2 py-1.5 text-sm';
</script>

{#snippet figure(f: SdlcFigure)}
	<span data-sdlc={f.key} class={f.reason ? 'text-ink-soft' : 'text-ink'}>{f.text}</span>
{/snippet}

{#snippet teamFigures(t: TeamView)}
	{#if t.line}<span data-team-line class="text-ink-soft">{t.line}</span>{/if}
	{#each t.figures as f (f.key)}
		<span class="text-ink-mute">{f.label} {@render figure(f)}</span>
	{/each}
{/snippet}

<Screen>
	{#snippet header()}
		<div class="flex flex-col gap-4 px-4 pt-4 md:px-8 md:pt-8">
			{#if onBack}
				<button
					type="button"
					onclick={onBack}
					class="text-ink-soft inline-flex cursor-pointer items-center gap-1 self-start bg-transparent text-sm"
				>
					<Icon name="alt-arrow-left" size={15} toneClass="text-ink-soft" /> Back to the workspace
				</button>
			{/if}
			<SectionHead kanji="納" eyebrow="Delivery · sprints and tickets" title={workspaceName} />
		</div>
	{/snippet}

	<div class="flex flex-col gap-10 px-4 pb-8 md:px-8">
		{#if error}
			<LoadError what="this workspace's delivery" {error} />
		{:else if view && view.boards.length === 0}
			<p class="text-sm text-ink-faint">No board has reported to this dōjō yet.</p>
		{:else if view}
			{#each view.boards as board (board.id)}
				<section data-board={board.id} aria-label="{board.name} sprints" class="flex min-w-0 flex-col gap-4">
					<h2 class="m-0 text-base text-ink">{board.name}</h2>
					<div class="flex flex-wrap gap-x-6 gap-y-1 text-sm">
						<span class="text-ink">This board</span>
						<span class="text-ink-mute">predictability {@render figure(board.predictability)}</span>
					</div>
					{#if board.team.line}
						<div data-team="project" class="flex flex-wrap gap-x-6 gap-y-1 text-sm">
							<span class="text-ink">Team</span>
							{@render teamFigures(board.team)}
						</div>
					{/if}

					{#if board.sprints.length === 0}
						<p class="text-sm text-ink-faint">This board has no sprints yet.</p>
					{:else}
						<div class="overflow-x-auto">
							<table class="min-w-[48rem] border-collapse">
								<thead>
									<tr>
										<th scope="col" class={th}>Sprint</th>
										<th scope="col" class={th}>Dates</th>
										{#each board.sprints[0].figures as f (f.key)}
											<th scope="col" class={th}>{f.label}</th>
										{/each}
										<th scope="col" class={th}>Team AI time</th>
										<th scope="col" class={th}>Team API cost</th>
									</tr>
								</thead>
								<tbody>
									{#each board.sprints as sprint (sprint.id)}
										{@const chosen = state.selectedFor(board.id) === sprint.id}
										<tr data-sprint={sprint.id} class="border-t border-paper-edge {chosen ? 'bg-paper-mute' : ''}">
											<th scope="row" class="{td} text-left font-normal">
												<button
													type="button"
													data-action="select"
													aria-pressed={chosen}
													class="cursor-pointer bg-transparent p-0 text-left text-ink underline"
													onclick={() => state.select(board.id, sprint.id)}>{sprint.name}</button
												>
												<span class="block text-xs text-ink-soft">{sprint.state}</span>
											</th>
											<td class="{td} text-ink-soft">{sprint.dates}</td>
											{#each sprint.figures as f (f.key)}
												<td class={td}>{@render figure(f)}</td>
											{/each}
											{#if sprint.team.figures.length}
												{#each sprint.team.figures as f (f.key)}
													<td class={td}>{@render figure(f)}</td>
												{/each}
											{:else}
												<td colspan="2" data-team-line class="{td} text-ink-soft">{sprint.team.line ?? '—'}</td>
											{/if}
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
						<p class="m-0 max-w-prose text-xs text-ink-soft">{TEAM_EXPLAINER}</p>
					{/if}

					{#if state.selectedFor(board.id)}
						<div class="flex flex-col gap-2 rounded border border-paper-edge p-4">
							{#if state.ticketsError}
								<p role="alert" class="m-0 text-sm text-danger">{state.ticketsError}</p>
							{:else if state.tickets === null}
								<p class="m-0 text-sm text-ink-mute">Reading tickets…</p>
							{:else if state.tickets.length === 0}
								<p class="m-0 text-sm text-ink-mute">This sprint lists no tickets.</p>
							{:else}
								<ul class="m-0 flex list-none flex-col gap-1 p-0" aria-label="Tickets">
									{#each state.tickets as t (t.key)}
										<li data-ticket={t.key} class="text-sm">
											<details>
												<summary class="flex cursor-pointer gap-3">
													<span class="mono text-xs text-ink-soft">{t.key}</span>
													{#if t.title}<span class="truncate text-ink">{t.title}</span>{/if}
													<span data-team-line class="text-xs text-ink-soft">{t.team.line ?? 'no team effort reported'}</span>
												</summary>
												{#if t.team.figures.length}
													<div data-team="ticket" class="flex flex-wrap gap-x-6 gap-y-1 py-2 pl-6">
														{#each t.team.figures as f (f.key)}
															<span class="text-ink-mute">{f.label} {@render figure(f)}</span>
														{/each}
													</div>
												{/if}
											</details>
										</li>
									{/each}
								</ul>
							{/if}
						</div>
					{/if}
				</section>
			{/each}
		{/if}
	</div>
</Screen>
