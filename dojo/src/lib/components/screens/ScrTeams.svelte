<script lang="ts">
	import { Screen } from '@sensei/ui';
	import { SectionHead, ListSection, Avatar, EmptyState, Banner, LoadError } from '$lib/components/kit';
	import type { TeamMetricsView, TeamMetricCell } from '$lib/team-metrics-view';

	// The Teams view (#307 R8, D38; mockup dojo2 ScrTeams / TeamDetail "By
	// developer"). Every member sees each of their teams' totals; a lead (their
	// teams) or an admin (every team) also sees one row per member. The database
	// decides whose numbers arrive; a dash is "nothing you may see", never a zero,
	// and a team number shows only once enough people sent it (k-anonymity). No team
	// is said in words; a failed read says it failed (never an empty table).
	// Presentational.
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
	const description = $derived(
		view?.perPerson
			? 'Each team, its totals, and the people on it — their numbers on the repositories you can read. Viewers never see this.'
			: "Each of your teams and its totals over the repositories you can read. People's own numbers stay with them and their leads."
	);
</script>

{#snippet summary(cells: TeamMetricCell[])}
	<span class="mono text-ink-mute text-xs md:hidden">
		{cells.map((c, j) => view!.columns[j].label + ' ' + c.text).join(' · ')}
	</span>
	{#each cells as cell (cell.key)}
		<span class="mono text-ink hidden text-sm md:inline">{cell.text}</span>
	{/each}
{/snippet}

<Screen>
	{#snippet header()}
		<div class="px-4 pt-4 md:px-8 md:pt-8">
			<SectionHead kanji="組" eyebrow={orgName + ' · teams'} title="Teams" count={view?.teams.length} {description} />
		</div>
	{/snippet}
	<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
		{#if error}
			<LoadError what="your teams" {error} />
		{:else if view && view.teams.length === 0}
			<EmptyState kanji="組" title="You are on no team yet.">
				An admin puts people on teams under Members &amp; Roles → Teams.
			</EmptyState>
		{:else if view}
			<Banner kanji="観" tone="neutral" title="These are counts, not transcripts.">
				Each number is what someone's sensei sent up — never their code, prompts or sessions. {view.window}.
				{view.totalsNote}
			</Banner>
			{#each view.teams as team (team.id)}
				<ListSection icon="users-group-rounded" title={team.name} count={team.memberCount}>
					<div class="border-paper-edge bg-paper-mute hidden gap-3 border-b px-4 py-2 md:grid {COLS}">
						<span class="text-ink-mute text-xs font-semibold uppercase">{view.perPerson ? 'Member' : 'Team'}</span>
						{#each view.columns as c (c.key)}
							<span class="text-ink-mute text-xs font-semibold uppercase">{c.label}</span>
						{/each}
						<span class="text-ink-mute text-right text-xs font-semibold uppercase">{view.perPerson ? 'Last' : ''}</span>
					</div>
					<div class="border-paper-edge bg-paper grid items-center gap-3 border-b px-4 py-3 {COLS}" data-team-total>
						<span class="text-ink text-sm font-medium">Team total</span>
						{@render summary(team.totals)}
						<span class="hidden md:inline"></span>
					</div>
					{#each team.members as m (m.principalId)}
						<div class="border-paper-edge grid items-center gap-3 border-b px-4 py-3 {COLS}" data-member={m.principalId}>
							<span class="flex min-w-0 items-center gap-3">
								<Avatar name={m.label} size={24} />
								<span class="text-ink min-w-0 truncate text-sm">{m.label}</span>
								{#if m.you}<span class="mono text-accent text-xs">you</span>{/if}
							</span>
							{@render summary(m.cells)}
							<span class="mono text-ink-faint hidden text-right text-xs md:inline">{m.last ?? '—'}</span>
						</div>
					{/each}
				</ListSection>
			{/each}
		{/if}
	</div>
</Screen>
