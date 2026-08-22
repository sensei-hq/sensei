<script lang="ts">
	import {
		SectionHead,
		Banner,
		Btn,
		Chip,
		ListSection,
		KanjiToken,
		ConfidenceBar,
		Enso,
		EmptyState
	} from '$lib/components/kit';
	import type { KitTriageGroup, KitCandidateDetail } from '$lib/components/kit/types';
	import { impactTone, needsSecondApproval } from '$lib/triage/candidates-view';
	import { createTriage } from '$lib/triage-state.svelte';
	import { untrack } from 'svelte';

	// The maintainer triage queue (mockup ScrTriage) — the richest maintainer
	// screen. Candidate learnings waiting on a maintainer, grouped by scope and
	// ranked by confidence, in a left column; the selected candidate's detail
	// (learning · cause · evidence · conflict · distribution scope + the
	// approve/revise/decline affordance) in a sticky right pane on desktop.
	// Presentational: the page supplies the groups + detail (kit fixtures this
	// chunk); the selection lives in the `triage-state` rune store. Below `md` the
	// detail pane is hidden and the grid collapses to the one-column list.
	// Degrades to an honest empty state when the queue is clear.
	let {
		orgName,
		groups = [],
		detail
	}: {
		orgName: string;
		groups?: KitTriageGroup[];
		detail: KitCandidateDetail;
	} = $props();

	// Seed the selection store ONCE from the props — a different org re-mounts this
	// screen (an /org/[slug]/triage navigation), so the focused candidate is
	// captured at mount (the same idiom as ScrOrgLadder).
	const triage = untrack(() => createTriage(groups));
</script>

<!-- The tracked-uppercase micro-label the detail pane reuses for each field. -->
{#snippet eyebrow(label: string)}
	<div
		class="text-ink-mute text-xs font-semibold uppercase"
		style="letter-spacing: 0.18em; margin-bottom: 4px"
	>
		{label}
	</div>
{/snippet}

<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
	<SectionHead eyebrow={orgName + ' · govern'} title="Triage" count={triage.all.length}>
		{#snippet right()}
			<Btn size="sm" variant="ghost" icon="tuning-2">My scopes</Btn>
		{/snippet}
	</SectionHead>

	<Banner
		kanji="門"
		tone="neutral"
		title="Candidate learnings waiting on a maintainer, grouped by scope and ranked by confidence."
	>
		You decide what becomes shared knowledge. High-impact and safety candidates need a second
		approval before they publish.
	</Banner>

	{#if triage.all.length}
		<div class="grid gap-4 items-start grid-cols-1 md:grid-cols-[1.1fr_1fr]">
			<!-- Left column — the scope-grouped, confidence-ranked candidate queue. -->
			<div class="flex flex-col gap-4">
				{#each groups as g (g.scope)}
					<ListSection icon="layers-minimalistic" title={g.scope} count={g.items.length}>
						{#each g.items as c (c.id)}
							{@const on = triage.isSelected(c.id)}
							{@const im = impactTone(c.impact)}
							<button
								type="button"
								onclick={() => triage.select(c.id)}
								aria-pressed={on}
								class="border-paper-edge flex w-full cursor-pointer items-center gap-3 border-b text-left {on
									? 'bg-accent-soft'
									: 'bg-transparent'}"
								style="padding: 12px 16px"
							>
								<KanjiToken char={c.kanji} size="base" toneClass="text-accent" w={20} />
								<div class="flex-1" style="min-width: 0">
									<div class="text-ink text-sm" style="line-height: 1.3">{c.title}</div>
									<div
										class="flex flex-wrap items-center gap-2"
										style="margin-top: 4px"
									>
										<span class="mono text-ink-faint text-xs">{c.origin}</span>
										{#if c.conflicts > 0}
											<Chip
												icon="danger-triangle"
												toneClass="text-warning"
												softClass="bg-warning-soft"
												edgeClass="border-warning-soft">{c.conflicts} conflict</Chip
											>
										{/if}
										{#if c.dups > 0}
											<Chip toneClass="text-ink-mute">{c.dups} dup</Chip>
										{/if}
										{#if c.impact === 'high' || c.impact === 'safety'}
											<Chip toneClass={im.text} softClass={im.soft} edgeClass={im.edge}
												>{c.impact}</Chip
											>
										{/if}
									</div>
								</div>
								<ConfidenceBar v={c.conf} w={56} />
							</button>
						{/each}
					</ListSection>
				{/each}
			</div>

			<!-- Right pane — the focused candidate's detail + decision affordance. -->
			{#if triage.current}
				{@const cur = triage.current}
				<div
					class="bg-paper-soft border-paper-edge hidden overflow-hidden rounded-lg border md:block"
					style="position: sticky; top: 0"
				>
					<div class="border-paper-edge flex items-center gap-3 border-b" style="padding: 16px">
						<KanjiToken char={cur.kanji} size="xl" toneClass="text-accent" />
						<div class="flex-1" style="min-width: 0">
							<div
								class="text-ink-mute text-xs font-semibold uppercase"
								style="letter-spacing: 0.18em"
							>
								candidate
							</div>
							<div class="text-ink text-sm font-medium">{cur.title}</div>
						</div>
						<Enso progress={cur.conf} size={54} label={Math.round(cur.conf * 100) + ''} />
					</div>
					<div class="flex flex-col gap-3" style="padding: 16px">
						<div>
							{@render eyebrow('Learning')}
							<div class="text-ink text-sm">{detail.learning}</div>
						</div>
						<div>
							{@render eyebrow('Cause')}
							<div class="text-ink-soft text-sm">{detail.cause}</div>
						</div>
						<div>
							{@render eyebrow('Evidence')}
							<div class="flex flex-col gap-1">
								{#each detail.evidence as e (e)}
									<div class="mono text-ink-soft text-xs">· {e}</div>
								{/each}
							</div>
						</div>
						{#if cur.conflicts > 0}
							<div
								class="bg-warning-soft border-warning-soft rounded-lg border"
								style="padding: 12px"
							>
								<div class="text-warning text-xs font-semibold uppercase" style="letter-spacing: 0.18em; margin-bottom: 4px">
									Conflict — the ladder settles it
								</div>
								<div class="text-ink-soft text-xs" style="text-decoration: line-through">
									{detail.conflict.loser}
								</div>
								<div class="text-ink text-sm font-medium" style="margin-top: 2px">
									{detail.conflict.winner}
								</div>
							</div>
						{/if}
						<div>
							{@render eyebrow('Distribution scope')}
							<div class="flex flex-wrap gap-2">
								{#each detail.scopes as s, i (s)}
									<Chip
										toneClass={i === 1 ? 'text-accent' : 'text-ink-mute'}
										softClass={i === 1 ? 'bg-accent-soft' : 'bg-paper-mute'}
										edgeClass={i === 1 ? 'border-accent-soft' : 'border-paper-edge'}>{s}</Chip
									>
								{/each}
							</div>
						</div>
					</div>
					<div
						class="bg-paper border-paper-edge flex flex-wrap items-center gap-2 border-t"
						style="padding: 12px 16px"
					>
						<Btn size="sm" icon="check-circle">Approve</Btn>
						<Btn size="sm" variant="ghost" icon="pen-2">Revise</Btn>
						<span class="flex-1"></span>
						<Btn size="sm" variant="ghost" icon="close-circle">Decline</Btn>
					</div>
					{#if needsSecondApproval(cur.impact)}
						<div
							class="bg-paper border-paper-edge text-ink-mute border-t text-xs"
							style="padding: 8px 16px"
						>
							Approving sends this to a second maintainer before it publishes.
						</div>
					{/if}
				</div>
			{/if}
		</div>
	{:else}
		<EmptyState kanji="静" title="The triage queue is clear.">
			Candidate learnings land here for a maintainer's decision. Nothing is waiting on you right
			now.
		</EmptyState>
	{/if}
</div>
