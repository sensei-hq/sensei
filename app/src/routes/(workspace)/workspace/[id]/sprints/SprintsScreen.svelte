<script lang="ts">
  // The Sprints screen (docs/spec/screen/workspace-sprints.md): one lane per
  // sprint board, side by side (D40).
  // PRESENTATIONAL: the numbers, words and requests are sprints-state's; this
  // renders them and reports clicks.
  import { PageHeader, Screen } from '$lib/components';
  import { replayHref } from '$lib/sessions-digest.js';
  import { usageCells, type SprintsState } from './sprints-state.svelte.js';

  let { screen, workspaceId }: { screen: SprintsState; workspaceId: string } = $props();

  const COLS = 'grid grid-cols-[minmax(12rem,1fr)_5rem_6rem_6rem_5rem_5rem_4rem] items-center gap-3';
  const button = 'rounded-md border border-paper-edge px-3 py-1.5 text-sm text-ink hover:bg-paper-mute';

  const when = (iso: string | null) =>
    iso
      ? new Date(iso).toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
      : '—';
</script>

{#snippet usageRow(label: string, u: import('$lib/types.js').TrackerUsage)}
  {@const c = usageCells(u)}
  <div class="flex flex-wrap gap-x-6 gap-y-1 text-sm">
    <span class="text-ink">{label}</span>
    <span class="text-ink-mute">sessions <span class="text-ink">{u.sessions}</span></span>
    <span class="text-ink-mute">active AI time <span class="text-ink">{c.active}</span></span>
    <span class="text-ink-mute">fresh tokens <span class="text-ink">{c.fresh}</span></span>
    <span class="text-ink-mute">out tokens <span class="text-ink">{c.out}</span></span>
    <span class="text-ink-mute">cost <span class="text-ink">{c.cost}</span></span>
  </div>
{/snippet}

<Screen>
  {#snippet header()}
    <PageHeader
      kanji="期"
      icon="i-solar-calendar-bold-duotone"
      eyebrow="Sprints"
      title={screen.boardTitle || 'Sprints'}
      description="Active AI time and tokens per sprint and ticket, from AI sessions whose branch or prompt names the ticket. Read from Jira; nothing is changed there."
    />
  {/snippet}
<section data-screen="sprints" class="flex flex-col">

  <div class="px-6 py-6">
    {#if screen.view === 'loading'}
      <p class="m-0 text-sm text-ink-soft">Reading sprints…</p>
    {:else if screen.view === 'not_connected'}
      <div data-view="not_connected" class="flex flex-col gap-2 text-sm text-ink">
        <p class="m-0">This workspace has no sprint board connected.</p>
        <a href="/workspace/{workspaceId}/integrations" class="self-start text-primary underline">Connect one in Integrations</a>
      </div>
    {:else if screen.view === 'daemon_error'}
      <div data-view="daemon_error" class="flex flex-col gap-3 rounded-md border border-paper-edge bg-paper-soft p-4">
        <p role="alert" class="m-0 text-sm text-danger">{screen.loadError}</p>
        <div>
          <button type="button" class={button} data-action="retry" onclick={() => screen.load()}>Retry</button>
        </div>
      </div>
    {:else}
      <div class="flex flex-col gap-6 md:flex-row">
        <div class="flex shrink-0 gap-4 overflow-x-auto">
          {#each screen.lanes as lane (lane.id)}
            <nav aria-label="{lane.title} sprints" data-lane={lane.id} class="flex shrink-0 flex-col gap-2 md:w-64">
              {#if screen.lanes.length > 1}
                <h2 class="m-0 font-heading text-sm text-ink">{lane.title}</h2>
              {/if}
              <ul class="m-0 flex list-none flex-col gap-1 p-0">
                {#each lane.rows as r (r.id)}
                  <li>
                    <button
                      type="button"
                      data-sprint={r.id}
                      aria-current={r.current ? 'true' : undefined}
                      class="flex w-full flex-col items-start gap-0.5 rounded-md border px-3 py-2 text-left {r.current
                        ? 'border-primary bg-paper-soft'
                        : 'border-transparent hover:bg-paper-mute'}"
                      onclick={() => screen.select(r.id)}
                    >
                      <span class="text-sm text-ink">{r.name}</span>
                      <span class="text-xs text-ink-mute">{r.state} · {r.dates}</span>
                      <span class="text-xs text-ink-soft">{r.tickets}</span>
                    </button>
                  </li>
                {:else}
                  <li class="text-sm text-ink-soft">The board has no sprints.</li>
                {/each}
              </ul>
            </nav>
          {/each}
        </div>

        <div class="flex min-w-0 flex-1 flex-col gap-4">
          {#if screen.ticket}
            {@const t = screen.ticket}
            <div data-view="ticket" class="flex flex-col gap-4">
              <div>
                <button type="button" class={button} data-action="back" onclick={() => screen.closeTicket()}
                  >Back to {screen.selected?.name ?? 'the sprint'}</button
                >
              </div>
              <div class="flex flex-col gap-1">
                <h2 class="m-0 font-heading text-lg text-ink">{t.task.key} · {t.task.summary ?? ''}</h2>
                <p class="m-0 text-xs text-ink-mute">
                  {t.task.status ?? '—'}{t.task.removed ? ' · no longer on the board' : ''}
                </p>
              </div>
              {@render usageRow('This ticket', t.usage)}

              <h3 class="m-0 text-sm text-ink">By repository</h3>
              {#if t.by_repo.length === 0}
                <p class="m-0 text-sm text-ink-soft">No AI sessions linked yet.</p>
              {:else}
                <ul class="m-0 flex list-none flex-col gap-2 p-0">
                  {#each t.by_repo as r (r.repository_id)}
                    <li>{@render usageRow(r.name, r.usage)}</li>
                  {/each}
                </ul>
              {/if}

              <h3 class="m-0 text-sm text-ink">Sessions</h3>
              {#if t.sessions.length === 0}
                <p class="m-0 text-sm text-ink-soft">No AI sessions linked yet.</p>
              {:else}
                <ul class="m-0 flex list-none flex-col gap-1 p-0">
                  {#each t.sessions as s (s.id)}
                    <li class="flex flex-wrap gap-3 text-sm">
                      <a href={replayHref(s.id)} data-session={s.id} class="text-primary underline">{when(s.started_at)}</a>
                      <span class="text-ink-mute">{screen.linksText(s)}</span>
                    </li>
                  {/each}
                </ul>
              {/if}
            </div>
          {:else if screen.selected}
            {@const strip = screen.strip}
            <h2 class="m-0 font-heading text-lg text-ink">{screen.selected.name}</h2>

            <div data-strip class="flex flex-col gap-2 rounded-lg border border-paper-edge bg-paper-soft p-4">
              {#if strip?.kind === 'reason'}
                <p class="m-0 text-sm text-ink-soft">{strip.text}</p>
              {:else if strip?.kind === 'window'}
                <p class="m-0 text-sm text-ink">{strip.total}</p>
                {#each strip.parts as part (part.label)}
                  <div class="flex flex-wrap items-baseline gap-x-6 gap-y-1 text-sm">
                    {#if part.label === 'Not linked' && part.sessions !== '0'}
                      <button
                        type="button"
                        class="bg-transparent p-0 text-ink underline"
                        aria-expanded={screen.unlinked !== null}
                        data-action="unlinked"
                        onclick={() => screen.toggleUnlinked()}>{part.label}</button
                      >
                    {:else}
                      <span class="text-ink">{part.label}</span>
                    {/if}
                    <span class="text-ink-mute">sessions <span class="text-ink">{part.sessions}</span></span>
                    <span class="text-ink-mute">active AI time <span class="text-ink">{part.active}</span></span>
                    <span class="text-ink-mute">fresh tokens <span class="text-ink">{part.fresh}</span></span>
                  </div>
                {/each}
              {/if}
            </div>

            {#if screen.unlinkedError}
              <p role="alert" class="m-0 text-sm text-danger">{screen.unlinkedError}</p>
            {/if}
            {#if screen.unlinked}
              <ul class="m-0 flex list-none flex-col gap-1 p-0" aria-label="Sessions not linked to a ticket">
                {#each screen.unlinked as u (u.id)}
                  <li class="flex flex-wrap gap-3 text-sm">
                    <a href={replayHref(u.id)} data-session={u.id} class="text-primary underline">{when(u.started_at)}</a>
                    <span class="text-ink-mute">{u.task || 'session'}</span>
                  </li>
                {/each}
              </ul>
            {/if}

            {#if screen.ticketError}
              <p role="alert" class="m-0 text-sm text-danger">{screen.ticketError}</p>
            {/if}

            {#if screen.tasksError}
              <p role="alert" class="m-0 text-sm text-danger">{screen.tasksError}</p>
            {:else if screen.tasks === null}
              <p class="m-0 text-sm text-ink-soft">Reading tickets…</p>
            {:else if screen.tasks.length === 0}
              <p class="m-0 text-sm text-ink-soft">This sprint lists no tickets.</p>
            {:else}
              <div class="overflow-x-auto">
                <div class="min-w-[40rem]">
                  <div aria-hidden="true" class="{COLS} border-b border-paper-edge px-2 pb-1 text-xs text-ink-mute">
                    <span>Ticket</span><span>Status</span><span>Sessions (branch/prompt)</span>
                    <span>Active AI time</span><span>Fresh tokens</span><span>Out tokens</span><span>Cost</span>
                  </div>
                  <ul class="m-0 list-none p-0">
                    {#each screen.tasks as t (t.key)}
                      {@const c = usageCells(t.usage)}
                      {@const moved = screen.movedTo(t)}
                      <li>
                        <button
                          type="button"
                          data-ticket={t.key}
                          class="{COLS} w-full rounded-md px-2 py-1.5 text-left text-sm hover:bg-paper-mute"
                          onclick={() => screen.openTicket(t.key)}
                        >
                          <span class="flex min-w-0 flex-col">
                            <span class="font-mono text-xs text-ink-mute">{t.key}</span>
                            <span class="truncate text-ink" title={t.summary ?? ''}
                              >{t.summary ?? ''}{#if moved}<span class="text-xs text-ink-soft"> · {moved}</span>{/if}</span
                            >
                          </span>
                          <span class="truncate text-ink-mute"><span class="sr-only">status </span>{t.status ?? '—'}</span>
                          <span><span class="sr-only">sessions </span><span data-cell="sessions">{c.sessions}</span></span>
                          <span><span class="sr-only">active AI time </span><span data-cell="active">{c.active}</span></span>
                          <span><span class="sr-only">fresh tokens </span><span data-cell="fresh">{c.fresh}</span></span>
                          <span><span class="sr-only">out tokens </span><span data-cell="out">{c.out}</span></span>
                          <span><span class="sr-only">cost </span><span data-cell="cost">{c.cost}</span></span>
                        </button>
                      </li>
                    {/each}
                  </ul>
                </div>
              </div>
              {#if screen.sharedNote}
                <p class="m-0 text-xs text-ink-soft">{screen.sharedNote}</p>
              {/if}
            {/if}
          {/if}
        </div>
      </div>
    {/if}
  </div>
</section>
</Screen>
