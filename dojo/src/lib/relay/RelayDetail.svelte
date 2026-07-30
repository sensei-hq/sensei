<script lang="ts">
	import { KanjiToken, PlanBar, PlanOutline, ListSection, EmptyState } from '$lib/components/kit';
	import { Toggle } from '@rokkit/ui';
	import type { KitTask } from '$lib/components/kit/types';
	import RelayStatusBadge from '$lib/components/RelayStatusBadge.svelte';
	import RelayAskCard from './RelayAskCard.svelte';
	import { relayInboxState } from './relay-inbox-state.svelte';
	import { relativeAge } from '$lib/triage/view';
	import * as m from '$lib/paraglide/messages';

	// The run detail (mockup RunDetail) — right panel of the two-panel inbox. Reads
	// the selected RelaySession straight from relayInboxState (mock-first; a per-run
	// realtime sub-state is extracted only when the real feed needs it). Header (観 ·
	// session id · meta · status) · progress · two tabs (Needs you / Plan) · AskCards
	// or the plan outline · activity + conversation. Pure over state.
	const st = relayInboxState;
	const run = $derived(st.selected);

	// Open (unanswered) asks drive the badge + the header/bar accent tone.
	const openCount = $derived(run ? run.asks.filter((a) => !st.answered[a.id]).length : 0);
	const tone = $derived(openCount ? 'text-accent' : 'text-ink-mute');

	// Progress: current phase = first not-done; the done/total numbers are the run's
	// own authoritative counts (its plan graph is summarized to one pip per phase).
	const activeIdx = $derived(run ? run.plan.phases.findIndex((p) => p.state !== 'done') : -1);
	const stages = $derived(run?.plan.phases.length ?? 0);
	const stage = $derived(activeIdx === -1 ? stages : activeIdx + 1);
	const pct = $derived(run && run.total ? Math.round((run.done / run.total) * 100) : 0);

	const sessionId = $derived((run?.id ?? '').slice(0, 8));
	const meta = $derived(
		[
			run?.project,
			run?.model,
			run?.startedAt ? relativeAge(run.startedAt) : null,
			run?.lastEventAt ? `last activity ${relativeAge(run.lastEventAt)}` : null
		]
			.filter((b): b is string => !!b)
			.join(' · ')
	);

	// Toggle options (rokkit segmented control). The open-ask count rides in the
	// "Needs you" label (Toggle has no separate badge slot).
	const TAB_OPTS = $derived([
		{ label: openCount ? `${m.detail_tab_needs()} · ${openCount}` : m.detail_tab_needs(), value: 'needs' },
		{ label: m.detail_tab_plan(), value: 'plan' }
	]);
	// Per-run tab pick — a new run falls back to its own default (needs if any, else
	// plan); a realtime refresh never clobbers the tab the user is on (pure $derived).
	let picks = $state<Record<string, string>>({});
	const tab = $derived(run ? (picks[run.id] ?? (openCount ? 'needs' : 'plan')) : 'needs');
	const pickTab = (id: string) => run && (picks = { ...picks, [run.id]: id });

	let selectedTaskId = $state<string | undefined>(undefined);
	const onSelectTask = (t: KitTask) => (selectedTaskId = selectedTaskId === t.id ? undefined : t.id);

	const answer = (askId: string, verdict: string) => st.answerAsk(askId, verdict);
</script>

{#if run}
	<div class="flex h-full min-h-0 flex-col">
		<!-- Sticky header (identity · meta · status · progress · tabs) — never scrolls. -->
		<div class="flex shrink-0 flex-col gap-5 px-8 pb-4 pt-8">
			<!-- Header: session identity + meta + live status. -->
		<div class="flex items-baseline gap-3">
			<KanjiToken char="観" size="lg" toneClass={tone} />
			<div style="min-width: 0; flex: 1">
				<div class="text-ink-mute text-xs font-medium uppercase" style="letter-spacing: 0.18em">
					{m.detail_session()} · {sessionId}
				</div>
				<h2 class="display text-ink text-lg font-normal" style="margin: 2px 0 0; line-height: 1.4">
					{run.title}
				</h2>
				{#if meta}
					<div class="mono text-ink-mute text-xs" style="margin-top: 3px">{meta}</div>
				{/if}
			</div>
			<RelayStatusBadge status={run.status} />
		</div>

		<!-- Progress: phase-of-phases + done/total + bar (accent while something waits). -->
		{#if stages}
			<div style="max-width: 560px">
				<div class="mb-2 flex items-center justify-between">
					<span class="mono text-ink-soft text-xs">
						{m.detail_phase({ stage, stages })}{run.phase ? ` · ${run.phase}` : ''}
					</span>
					<span class="mono text-ink-soft text-xs">{m.detail_tasks_pct({ done: run.done, total: run.total, pct })}</span>
				</div>
				<PlanBar {pct} tone={openCount ? 'bg-accent' : 'bg-ink'} />
			</div>
		{/if}

			<!-- Two tabs: Needs you · Plan. -->
			<div class="border-paper-edge border-b pb-3">
				<Toggle options={TAB_OPTS} value={tab} onchange={(v) => pickTab(v as string)} size="sm" label={m.detail_tab_needs()} />
			</div>
		</div>

		<!-- Scrolling body: asks / plan + activity (the only scroll region). -->
		<div class="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-8 pb-8">
			{#if tab === 'needs'}
			{#if run.asks.length}
				<div class="flex flex-col gap-3">
					{#each run.asks as ask (ask.id)}
						<RelayAskCard {ask} verdict={st.answered[ask.id]} onanswer={answer} />
					{/each}
				</div>
			{:else}
				<EmptyState kanji="静" title={m.detail_empty_title()}>{m.detail_empty_body()}</EmptyState>
			{/if}
		{:else if stages}
			<ListSection
				icon="layers-minimalistic"
				iconToneClass="text-ink-mute"
				title={m.detail_tab_plan()}
				count={m.detail_plan_count({ stages, total: run.total })}
			>
				{#if run.goal}
					<div class="border-paper-edge mono text-ink-mute border-b px-4 py-3 text-xs">
						{m.detail_goal({ goal: run.goal })}
					</div>
				{/if}
				<PlanOutline plan={run.plan} selectedId={selectedTaskId} onSelect={onSelectTask} />
			</ListSection>
		{:else}
			<EmptyState kanji="継" title={m.detail_plan_empty_title()}>{m.detail_plan_empty_body()}</EmptyState>
		{/if}

		<!-- Activity + Conversation (mock carries neither yet — honest empty). -->
		<div class="grid grid-cols-1 items-start gap-4 md:grid-cols-2">
			<ListSection icon="pulse" iconToneClass="text-ink-mute" title={m.detail_activity()}>
				<div class="text-ink-mute px-4 py-3 text-sm">{m.detail_activity_empty()}</div>
			</ListSection>
			<ListSection icon="chat-round-line" iconToneClass="text-ink-mute" title={m.detail_conversation()}>
				<div class="text-ink-mute px-4 py-3 text-sm">{m.detail_conversation_empty()}</div>
			</ListSection>
			</div>
		</div>
	</div>
{/if}
