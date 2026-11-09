<script lang="ts">
  // Relay · the crew screen (D90; CONTRACT §1a–§1d, §1f, §1g). One screen, two
  // places: the workspace window draws it pre-filtered to its workspace, the main
  // window draws every crew and offers the repository/workspace filters. The
  // difference is the data and the base of a task's link, never the layout.
  //
  // The loader reads; this screen draws what it read and hands every write to
  // `RelayState`, which is the only thing that reads again. A failed read is a
  // Notice with a Retry — never an empty crew, never "No crew yet" (§1f).
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { Loading, Notice, ShellScreen } from '@sensei/ui';
  import { failureReason } from '$lib/api.js';
  import { KANJI_ICONS } from '$lib/glyph-vocabulary';
  import { useGlyphMode } from '$lib/glyphs';
  import type { InfoSlot } from '@rokkit/app';
  import type { RelayCrew, RelayLane } from '$lib/relay-api.js';
  import type { RelayState } from './relay-state.svelte.js';
  import {
    crewGroups,
    crewRepoLabel,
    hhmm,
    laneState,
    providerLabel,
    since,
    taskHref,
    todayLine,
  } from './relay-format.js';
  import DecisionRow from './DecisionRow.svelte';
  import MessageRow from './MessageRow.svelte';
  import ComposeBand from './ComposeBand.svelte';
  import QueueRow from './QueueRow.svelte';
  import RoleMark from './RoleMark.svelte';

  let {
    state,
    viewId,
    base,
    context,
    filtered = false,
  }: {
    state: RelayState;
    viewId: string;
    /** Where this window's Relay lives: `/workspace/<id>/relay`, or `/relay`. */
    base: string;
    /** The line after the title: the workspace and crew, or the crews it shows. */
    context: string;
    /** Draw the repository/workspace filter band (the main window does). */
    filtered?: boolean;
  } = $props();

  const mode = useGlyphMode();

  const today = $derived(state.now ? todayLine(state.now.today) : null);
  const triageTotal = $derived((state.now?.triage ?? []).reduce((n, t) => n + t.count, 0));
  const triageFrom = $derived(
    (state.now?.triage ?? [])
      .map((t) => (t.provider === 'manual' ? 'you added it' : `${providerLabel(t.provider)} ${t.sprint ?? ''}`.trim()))
      .join(' · '),
  );

  /** "rust-build" → "Rust build". */
  const resourceName = (id: string): string => {
    const words = id.replace(/[-_]/g, ' ').trim();
    return words.charAt(0).toUpperCase() + words.slice(1);
  };

  const footerLine = $derived.by(() => {
    const r = state.now?.resources;
    if (!r) return '';
    const build = r.build_lease
      ? `${resourceName(r.build_lease.resource)} · ${r.build_lease.lane_code} · ${since(r.build_lease.granted_at)}`
      : 'no build running';
    const disk = r.disk_free_gb == null ? 'Disk not reported' : `Disk ${r.disk_free_gb} GB free`;
    const swap =
      r.swap_free_gb == null
        ? 'Swap not reported'
        : `Swap ${r.swap_free_gb} GB free (floor ${r.swap_floor_gb} GB)`;
    return `${build} · ${disk} · ${swap}`;
  });

  /** The queue's tab carries its count, and says nothing when it could not be read. */
  const queueLabel = $derived(state.queueCount == null ? 'Queue' : `Queue · ${state.queueCount}`);

  /** The notice a failed crew read draws: a fault, its reason, and a Retry. */
  const failure = $derived(
    state.nowError
      ? {
          detail: failureReason(state.nowError),
          code: state.nowError.status === 0 ? 'daemon_unreachable' : `http_${state.nowError.status}`,
        }
      : null,
  );

  // ─── The main window's filters (CONTRACT §1a) ─────────────────────
  const filtersNow = $derived(state.now?.facets ?? { repos: [], workspaces: [] });
  const current = (key: 'repo' | 'workspace') => page.url.searchParams.get(key) ?? '';

  function narrow(key: 'repo' | 'workspace', value: string): void {
    const url = new URL(page.url);
    if (value) url.searchParams.set(key, value);
    else url.searchParams.delete(key);
    // The search change is this route's to answer: `goto` re-runs its load, and
    // nothing above it depends on the filter (N9).
    void goto(`${url.pathname}${url.search}`);
  }

  const shortRepo = (repo: string) => repo.replace(/^github\.com\//, '');

  const framing = {
    label: 'What Relay is',
    title: 'Your crew of assistant sessions',
    text: 'Relay is the hand-off: you assign a task, a lane works it, you read what came back and answer only what needs you.',
  };

  /** The lanes divided by crew in the main window, flat in a workspace. */
  const grouped = $derived(
    filtered && state.now
      ? crewGroups(state.now.crews, state.now.lanes)
      : state.now
        ? state.now.crews.slice(0, 1).map((crew) => ({ crew, lanes: state.now!.lanes }))
        : [],
  );

  const crewInfo = $derived<InfoSlot[] | null>(
    state.hasCrew
      ? [
          {
            label: 'What Relay is',
            title: 'Your crew of assistant sessions',
            text: 'Relay is the hand-off: you assign a task, a lane works it, you read what came back and answer only what needs you.',
          },
          {
            label: 'How it works',
            steps: [
              { kanji: '手', icon: KANJI_ICONS['手'], title: 'Assign', text: 'The coordinator puts the next task on a lane.' },
              { kanji: '作', icon: KANJI_ICONS['作'], title: 'Work', text: 'The lane works, reports, and asks when it must.' },
              { kanji: '見', icon: KANJI_ICONS['見'], title: 'Read', text: 'You read the summary and answer the open question.' },
            ],
          },
          { label: 'Today', value: today?.value ?? '', text: today?.text ?? '' },
        ]
      : null,
  );

  /** The two lines a session pastes to register (CONTRACT §1b). */
  const pasteLines = ['register with sensei as coordinator', 'register with sensei as contributor'];
  async function copy(text: string): Promise<void> {
    await navigator.clipboard?.writeText(text);
  }
</script>

{#snippet noCrewInfo()}
  <div class="flex flex-col gap-2">
    <p class="text-sm font-medium m-0" data-no-crew>No crew yet</p>
    <p class="text-sm text-ink-soft m-0">
      Register a session in this repository as the coordinator, then the contributors it invites.
    </p>
    {#each pasteLines as line (line)}
      <div class="flex items-center gap-2" data-paste-line>
        <code class="text-xs rounded-sm px-2 py-1 bg-paper-soft" data-paste-text>{line}</code>
        <button type="button" class="text-xs underline" data-paste-copy onclick={() => void copy(line)}>Copy</button>
      </div>
    {/each}
    <p class="text-xs text-ink-faint m-0">Listening · 0 of 2 to start</p>
  </div>
{/snippet}

{#snippet filterBand()}
  <div class="flex flex-wrap items-center gap-4">
    <label class="flex items-center gap-2 text-xs text-ink-mute">
      <span>Repository</span>
      <select
        data-relay-filter-repo
        class="rounded-sm border border-paper-edge bg-paper py-1 px-2 text-sm text-ink"
        value={current('repo')}
        onchange={(e) => narrow('repo', e.currentTarget.value)}
      >
        <option value="">All repositories</option>
        {#each filtersNow.repos as repo (repo)}
          <option value={repo}>{shortRepo(repo)}</option>
        {/each}
      </select>
    </label>
    <label class="flex items-center gap-2 text-xs text-ink-mute">
      <span>Workspace</span>
      <select
        data-relay-filter-workspace
        class="rounded-sm border border-paper-edge bg-paper py-1 px-2 text-sm text-ink"
        value={current('workspace')}
        onchange={(e) => narrow('workspace', e.currentTarget.value)}
      >
        <option value="">All workspaces</option>
        {#each filtersNow.workspaces as ws (ws.id)}
          <option value={ws.id}>{ws.name}</option>
        {/each}
      </select>
    </label>
  </div>
{/snippet}

<div data-screen="relay">
  <ShellScreen
    {viewId}
    glyphs={mode()}
    mark={{ kanji: '継', icon: KANJI_ICONS['継'] }}
    title="Relay"
    {context}
    search={false}
    toolbar={{
      add: [
        { id: 'now', label: 'Now', type: 'toggle', active: state.tab === 'now', onclick: () => (state.tab = 'now') },
        { id: 'queue', label: queueLabel, type: 'toggle', active: state.tab === 'queue', onclick: () => state.openQueue() },
        ...(state.hasCrew
          ? [
              {
                id: 'compose',
                label: 'Message the crew',
                icon: 'i-solar-chat-round-line-bold-duotone',
                onclick: () => (state.composing = !state.composing),
              },
            ]
          : []),
      ],
      // With no crew the info band is the whole screen and cannot be closed (§1b);
      // a failed read has no crew to frame, so it draws no info band at all.
      remove: state.hasCrew && !failure ? [] : ['info'],
    }}
    info={failure ? undefined : (crewInfo ?? [framing, noCrewInfo])}
    filters={filtered ? filterBand : undefined}
  >
    {#snippet footer()}
      {#if footerLine}<p class="text-xs text-ink-mute m-0" data-relay-footer>{footerLine}</p>{/if}
    {/snippet}

    {#if state.composing && state.now}
      <ComposeBand
        crews={state.now.crews}
        lanes={state.now.lanes}
        send={(body) => state.send(body)}
        onclose={() => (state.composing = false)}
      />
    {/if}

    {#if state.reading}
      <Loading />
    {:else if failure}
      <div class="p-4">
        <Notice
          data-relay-error
          tone="fault"
          size="panel"
          title="Couldn't read the crew"
          detail={failure.detail}
          code={failure.code}
          primary={{ label: 'Retry', name: 'retry', onclick: () => void state.retryNow() }}
        />
      </div>
    {:else if state.now && state.hasCrew && state.tab === 'now'}
      {#if state.now.decisions.length > 0 || triageTotal > 0 || state.now.person_messages.length > 0}
        <section class="flex flex-col gap-2 p-4 pb-0" data-relay-section="needs-you">
          <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">Needs you</h2>
          {#each state.now.decisions as decision (decision.id)}
            <DecisionRow
              {decision}
              href={decision.task_id ? taskHref(base, decision.crew_id, decision.task_id) : null}
              onanswer={(option) => state.answer(decision, option)}
            />
          {/each}
          {#each state.now.person_messages as message (message.id)}
            <MessageRow
              {message}
              href={message.task_id ? taskHref(base, message.crew_id, message.task_id) : null}
            />
          {/each}
          {#if triageTotal > 0}
            <button
              type="button"
              class="text-left text-sm text-ink-soft hover:text-ink"
              data-triage-line
              onclick={() => state.openQueue()}
            >
              {triageTotal} to triage — from {triageFrom}
            </button>
          {/if}
        </section>
      {/if}

      <section class="flex flex-col gap-2 p-4 pb-0" data-relay-section="lanes">
        <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">Lanes · {state.now.lanes.length}</h2>
        <!-- The workspace window draws one crew's lanes flat; the main window divides
             them by crew, because a lane code is unique per crew only (§1c). -->
        {#each grouped as group (group.crew.id)}
          <div data-crew-group={group.crew.id}>
            <p class="text-xs text-ink-faint m-0" data-crew-heading>{crewRepoLabel(group.crew)}</p>
            {#each group.lanes as lane (lane.id)}
              {@const lost = laneState(lane)}
              <div class="border-t border-paper-edge py-2" data-lane-row={lane.code}>
                <div class="flex items-center gap-2" data-row-line="1">
                  <RoleMark role={lane.role} />
                  <span class="font-medium text-sm">{lane.title ?? lane.code}</span>
                  <span class="text-xs text-ink-mute">{lane.code}</span>
                  <span class="text-xs text-ink-faint">
                    {lane.family}{lane.version ? ` ${lane.version}` : ''} · {lane.model ?? 'model not reported'}
                  </span>
                  <span class="ml-auto text-xs rounded-sm px-2 py-1 bg-paper-soft text-ink-soft" data-state>{lost}</span>
                  {#if lost === 'lease lost' && lane.lease_expires_at}
                    <span class="text-xs text-warning">at {hhmm(lane.lease_expires_at)}</span>
                  {/if}
                </div>
                <div class="text-sm text-ink-soft" data-row-line="2">
                  {#if lane.task}
                    <a class="hover:underline" href={taskHref(base, lane.crew_id, lane.task.id)}>{lane.task.title}</a>
                    <span class="text-xs text-ink-faint"> · {since(lane.task.state_changed_at)}</span>
                  {:else}
                    Nothing assigned
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/each}
      </section>

      {#if state.now.completed_today.length > 0}
        <section class="flex flex-col gap-2 p-4" data-relay-section="completed">
          <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">
            Completed today · {state.now.completed_today.length}
          </h2>
          {#each state.now.completed_today as row (row.task_id)}
            <div class="flex items-center gap-2 text-sm" data-completed-row>
              <span class="text-xs text-ink-faint tabular-nums">{hhmm(row.completed_at)}</span>
              <a class="hover:underline" href={taskHref(base, row.crew_id, row.task_id)}>{row.title}</a>
              <span class="text-xs text-ink-mute">{row.lane_title ?? row.lane_code}</span>
              <span class="ml-auto text-xs rounded-sm px-2 py-1 bg-paper-soft text-ink-soft" data-state>{row.outcome}</span>
            </div>
          {/each}
        </section>
      {/if}
    {:else if state.now && state.hasCrew && state.tab === 'queue'}
      {#if state.readingQueue}
        <Loading />
      {:else if state.queueError}
        <div class="p-4">
          <Notice
            data-relay-queue-error
            tone="fault"
            size="panel"
            title="Couldn't read the queue"
            detail={failureReason(state.queueError)}
            code={state.queueError.status === 0 ? 'daemon_unreachable' : `http_${state.queueError.status}`}
            primary={{ label: 'Retry', name: 'retry', onclick: () => void state.retryQueue() }}
          />
        </div>
      {:else if state.queue}
        <section class="flex flex-col gap-2 p-4 pb-0" data-relay-section="triage">
          <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">Triage · {state.queue.triage.length}</h2>
          {#if state.addError}
            <Notice size="inline" tone="fault" title={state.addError} data-add-error />
          {/if}
          {#each state.queue.triage as item (item.item_id ?? item.ticket?.key)}
            <QueueRow
              id={item.item_id ?? item.ticket?.key ?? ''}
              title={item.title}
              kind={item.kind}
              tags={item.tags}
              sprint={item.sprint}
              addedByYou={item.added_by_you}
              ticket={item.ticket}
              onadd={() => state.addToBacklog(item)}
            />
          {/each}
        </section>

        <section class="flex flex-col gap-2 p-4" data-relay-section="backlog">
          <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">Backlog · {state.queue.backlog.length}</h2>
          {#each [...state.queue.backlog].sort((a, b) => a.position - b.position) as item (item.item_id)}
            <QueueRow
              id={item.item_id}
              title={item.title}
              kind={item.kind}
              tags={item.tags}
              sprint={item.sprint}
              addedByYou={item.added_by_you}
              ticket={item.ticket}
            />
          {/each}
        </section>
      {/if}
    {/if}
  </ShellScreen>
</div>
