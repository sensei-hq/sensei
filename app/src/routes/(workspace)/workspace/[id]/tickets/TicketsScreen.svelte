<script lang="ts">
  // The Tickets screen (docs/spec/screen/workspace-tickets.md).
  // PRESENTATIONAL: the numbers, words and requests are tickets-state's; this
  // renders them and reports clicks.
  import { Select } from '@rokkit/ui';
  import { PageHeader, Screen } from '$lib/components';
  import { replayHref } from '$lib/sessions-digest.js';
  import { usageCells, type TicketsState } from './tickets-state.svelte.js';

  let { screen, workspaceId }: { screen: TicketsState; workspaceId: string } = $props();

  const COLS = 'grid grid-cols-[minmax(12rem,1fr)_7rem_5rem_6rem_6rem_5rem_5rem_5rem_4rem] items-center gap-3';
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
      kanji="札"
      icon="i-solar-ticket-bold-duotone"
      eyebrow="Tickets"
      title={screen.boardTitle || 'Tickets'}
      description="Active AI time and tokens per ticket and sprint, from AI sessions whose branch or prompt names the ticket. Read from Jira; nothing is changed there."
    />
  {/snippet}
<section data-screen="tickets" class="flex flex-col">

  <div class="px-6 py-6">
    {#if screen.view === 'loading'}
      <p class="m-0 text-sm text-ink-soft">Reading tickets…</p>
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
      <div class="flex min-w-0 flex-col gap-4">
        {#if screen.ticket}
          {@const t = screen.ticket}
          {@const time = usageCells(t.usage)}
          <div data-view="ticket" class="flex flex-col gap-4">
            <div>
              <button type="button" class={button} data-action="back" onclick={() => screen.closeTicket()}
                >{screen.backLabel}</button
              >
            </div>
            <div class="flex flex-col gap-1">
              <h2 class="m-0 font-heading text-lg text-ink">{t.task.key} · {t.task.summary ?? ''}</h2>
              <p class="m-0 text-xs text-ink-mute">
                {t.task.status ?? '—'}{t.task.removed ? ' · no longer on the board' : ''}
              </p>
            </div>
            {@render usageRow('This ticket', t.usage)}
            <div class="flex flex-wrap gap-x-6 gap-y-1 text-sm">
              <span class="text-ink">Time</span>
              <span class="text-ink-mute">Claude <span class="text-ink" data-time="claude">{time.claude}</span></span>
              <span class="text-ink-mute">you <span class="text-ink" data-time="you">{time.you}</span></span>
              <span class="text-ink-mute">session time <span class="text-ink" data-time="wall">{time.wall}</span></span>
              <span class="text-ink-mute"
                >cycle time <span class="text-ink" data-time="cycle">{time.cycle}</span>{#if t.usage.first_at && t.usage.last_at}<span
                    class="text-ink-soft"> · {when(t.usage.first_at)} → {when(t.usage.last_at)}</span
                  >{/if}</span
              >
            </div>

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
        {:else}
          {#if screen.ticketError}
            <p role="alert" class="m-0 text-sm text-danger">{screen.ticketError}</p>
          {/if}
          <div class="flex min-w-0 flex-col gap-6 xl:flex-row">
            {#each screen.lanes as lane (lane.id)}
              <section
                data-lane={lane.id}
                aria-label="{lane.title} tickets"
                class="flex min-w-0 flex-1 flex-col gap-4"
              >
                {#if screen.lanes.length > 1}
                  <h2 class="m-0 font-heading text-base text-ink">{lane.title}</h2>
                {/if}
              <div class="flex flex-wrap items-center gap-x-6 gap-y-2">
                <label class="flex items-center gap-2 text-sm text-ink-mute">
                  <span>Sprint</span>
                  <div class="min-w-[14rem]" data-filter="sprint">
                    <Select
                      filterable
                      items={lane.filterItems}
                      value={lane.filter}
                      onchange={(v) => lane.setFilter(String(v))}
                    />
                  </div>
                </label>
                <span class="flex items-center gap-2 text-sm text-ink">
                  <input type="checkbox" id="tickets-ai-only-{lane.id}" data-filter="ai-only" bind:checked={lane.aiOnly} />
                  <label for="tickets-ai-only-{lane.id}">Only tickets with AI sessions</label>
                </span>
                <p data-count class="m-0 text-sm text-ink-mute">{lane.countLine}</p>
              </div>

              {#if lane.strip}
                {@const strip = lane.strip}
                <div data-strip class="flex flex-col gap-2 rounded-lg border border-paper-edge bg-paper-soft p-4">
                  {#if strip.kind === 'reason'}
                    <p class="m-0 text-sm text-ink-soft">{strip.text}</p>
                  {:else}
                    <p class="m-0 text-sm text-ink">{strip.total}</p>
                    {#each strip.parts as part (part.label)}
                      <div class="flex flex-wrap items-baseline gap-x-6 gap-y-1 text-sm">
                        {#if part.label === 'Not linked' && part.sessions !== '0'}
                          <button
                            type="button"
                            class="bg-transparent p-0 text-ink underline"
                            aria-expanded={lane.unlinked !== null}
                            data-action="unlinked"
                            onclick={() => lane.toggleUnlinked()}>{part.label}</button
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
              {/if}

              {#if lane.unlinkedError}
                <p role="alert" class="m-0 text-sm text-danger">{lane.unlinkedError}</p>
              {/if}
              {#if lane.unlinked}
                <ul class="m-0 flex list-none flex-col gap-1 p-0" aria-label="Sessions not linked to a ticket">
                  {#each lane.unlinked as u (u.id)}
                    <li class="flex flex-wrap gap-3 text-sm">
                      <a href={replayHref(u.id)} data-session={u.id} class="text-primary underline">{when(u.started_at)}</a>
                      <span class="text-ink-mute">{u.task || 'session'}</span>
                    </li>
                  {/each}
                </ul>
              {/if}


              {#if lane.tasksError}
                <p role="alert" class="m-0 text-sm text-danger">{lane.tasksError}</p>
              {:else if lane.tasks === null}
                <p class="m-0 text-sm text-ink-soft">Reading tickets…</p>
              {:else if lane.visibleTasks.length === 0}
                <p class="m-0 text-sm text-ink-soft">No tickets to show.</p>
              {:else}
                <div class="overflow-x-auto">
                  <div class="min-w-[52rem]">
                    <div aria-hidden="true" class="{COLS} border-b border-paper-edge px-2 pb-1 text-xs text-ink-mute">
                      <span>Ticket</span><span>Sprint</span><span>Status</span><span>Sessions (branch/prompt)</span>
                      <span>Active AI time</span><span>Cycle time</span><span>Fresh tokens</span><span>Out tokens</span><span>Cost</span>
                    </div>
                    <ul class="m-0 list-none p-0">
                      {#each lane.visibleTasks as t (t.key)}
                        {@const c = usageCells(t.usage)}
                        <li>
                          <button
                            type="button"
                            data-ticket={t.key}
                            class="{COLS} w-full rounded-md px-2 py-1.5 text-left text-sm hover:bg-paper-mute"
                            onclick={() => screen.openTicket(lane, t.key)}
                          >
                            <span class="flex min-w-0 flex-col">
                              <span class="font-mono text-xs text-ink-mute">{t.key}</span>
                              <span class="truncate text-ink" title={t.summary ?? ''}>{t.summary ?? ''}</span>
                            </span>
                            <span class="truncate text-ink-mute"><span class="sr-only">sprint </span><span data-cell="sprint">{lane.sprintOf(t)}</span></span>
                            <span class="truncate text-ink-mute"><span class="sr-only">status </span>{t.status ?? '—'}</span>
                            <span><span class="sr-only">sessions </span><span data-cell="sessions">{c.sessions}</span></span>
                            <span><span class="sr-only">active AI time </span><span data-cell="active">{c.active}</span></span>
                            <span><span class="sr-only">cycle time </span><span data-cell="cycle">{c.cycle}</span></span>
                            <span><span class="sr-only">fresh tokens </span><span data-cell="fresh">{c.fresh}</span></span>
                            <span><span class="sr-only">out tokens </span><span data-cell="out">{c.out}</span></span>
                            <span><span class="sr-only">cost </span><span data-cell="cost">{c.cost}</span></span>
                          </button>
                        </li>
                      {/each}
                    </ul>
                  </div>
                </div>
                {#if lane.sharedNote}
                  <p class="m-0 text-xs text-ink-soft">{lane.sharedNote}</p>
                {/if}
              {/if}
              </section>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</section>
</Screen>
