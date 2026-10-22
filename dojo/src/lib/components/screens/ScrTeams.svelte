<script lang="ts">
	import { Screen } from '@sensei/ui';
	import { SectionHead, ListSection, Avatar, EmptyState, Banner } from '$lib/components/kit';
	import type { TeamMetricsView } from '$lib/team-metrics-view';

	// The lead's team view (#307 R8, D38; mockup dojo2 ScrTeams / TeamDetail "By
	// developer"): each team the lead is on, each member's key numbers across the
	// repositories the lead can read. The database decides whose numbers arrive; a
	// dash is "nothing you may see", never a zero. A lead on no team is told so; a
	// failed read says it failed (never an empty table). Presentational.
	let {
		orgName,
		view,
		error
	}: {
		orgName: string;
		view: TeamMetricsView | null;
		error: string | null;
	} = $props();

	const COLS = 'grid-cols-[minmax(0,1fr)_auto] md:grid-cols-[minmax(0,1.6fr)_repeat(4,minmax(0,1fr))_minmax(0,0.8fr)]';
</script>

<Screen>
	{#snippet header()}
		<div class="px-4 pt-4 md:px-8 md:pt-8">
			<SectionHead
				kanji="組"
				eyebrow={orgName + ' · lead'}
				title="Teams"
				count={view?.teams.length}
				description="The people you share a team with, one row each — their numbers on the repositories you can read. Viewers never see this; they see the workspace's totals."
			/>
		</div>
	{/snippet}
	<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
		{#if error}
			<p role="alert" class="text-sm text-danger m-0">Could not load your teams — {error}</p>
		{:else if view && view.teams.length === 0}
			<EmptyState kanji="組" title="You are on no team yet.">
				A lead sees the people on their own teams. An admin puts you on one under Members &amp; Roles → Teams.
			</EmptyState>
		{:else if view}
			<Banner kanji="観" tone="neutral" title="These are counts, not transcripts.">
				Each number is what a person's sensei sent up — never their code, prompts or sessions.
				{view.window}.
			</Banner>
			{#each view.teams as team (team.id)}
				<ListSection icon="users-group-rounded" title={team.name} count={team.members.length}>
					<div class="border-paper-edge bg-paper-mute hidden gap-3 border-b px-4 py-2 md:grid {COLS}">
						<span class="text-ink-mute text-xs font-semibold uppercase">Member</span>
						{#each view.columns as c (c.key)}
							<span class="text-ink-mute text-xs font-semibold uppercase">{c.label}</span>
						{/each}
						<span class="text-ink-mute text-right text-xs font-semibold uppercase">Last</span>
					</div>
					{#each team.members as m (m.principalId)}
						<div class="border-paper-edge grid items-center gap-3 border-b px-4 py-3 {COLS}" data-member={m.principalId}>
							<span class="flex min-w-0 items-center gap-3">
								<Avatar name={m.label} size={24} />
								<span class="text-ink min-w-0 truncate text-sm">{m.label}</span>
								{#if m.you}<span class="mono text-accent text-xs">you</span>{/if}
							</span>
							{#each m.cells as cell, i (cell.key)}
								<span class="mono text-ink hidden text-sm md:inline">{cell.text}</span>
								{#if i === 0}
									<span class="mono text-ink-mute text-xs md:hidden">
										{m.cells.map((c, j) => view.columns[j].label + ' ' + c.text).join(' · ')}
									</span>
								{/if}
							{/each}
							<span class="mono text-ink-faint hidden text-right text-xs md:inline">{m.last ?? '—'}</span>
						</div>
					{/each}
				</ListSection>
			{/each}
		{/if}
	</div>
</Screen>
