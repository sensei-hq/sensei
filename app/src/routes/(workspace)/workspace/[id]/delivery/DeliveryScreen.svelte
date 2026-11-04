<script lang="ts">
  // The Delivery Metrics screen (docs/spec/daemon/tracker-delivery-metrics.md D2).
  // PRESENTATIONAL: the numbers, words and requests are delivery-state's; this
  // renders them and reports clicks.
  import { PageHeader, Screen } from '$lib/components';
  import type { SdlcFigure } from '$lib/sdlc-figures.js';
  import type { DeliveryState } from './delivery-state.svelte.js';

  let { screen, workspaceId }: { screen: DeliveryState; workspaceId: string } = $props();

  const button = 'rounded-md border border-paper-edge px-3 py-1.5 text-sm text-ink hover:bg-paper-mute';
  const th = 'px-2 pb-1 text-left text-xs font-normal text-ink-soft';
  const td = 'px-2 py-1.5 text-sm';
</script>

{#snippet figureCell(f: SdlcFigure)}
  <td class={td}><span data-sdlc={f.key} class={f.reason ? 'text-ink-soft' : 'text-ink'}>{f.text}</span></td>
{/snippet}

{#snippet projectRow(kind: string, label: string, figures: SdlcFigure[])}
  <div data-delivery-project={kind} class="flex flex-wrap gap-x-6 gap-y-1 text-sm">
    <span class="text-ink">{label}</span>
    {#each figures as f (f.key)}
      <span class="text-ink-mute"
        >{f.label} <span data-sdlc={f.key} class={f.reason ? 'text-ink-soft' : 'text-ink'}>{f.text}</span></span
      >
    {/each}
  </div>
{/snippet}

<Screen>
  {#snippet header()}
    <PageHeader
      kanji="納"
      icon="i-solar-delivery-bold-duotone"
      eyebrow="Delivery Metrics"
      title="Delivery Metrics"
      description="How each sprint went: commitment kept, scope that moved, carry-over and velocity, read from Jira, with the AI work in your sessions behind it. Team figures are in the dōjō."
    />
  {/snippet}
  <section data-screen="delivery" class="flex flex-col">
    <div class="px-6 py-6">
      {#if screen.view === 'loading'}
        <p class="m-0 text-sm text-ink-mute">Reading sprints…</p>
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
        <div class="flex min-w-0 flex-col gap-10">
          {#each screen.boards as board (board.id)}
            {@const rows = board.rows}
            <section data-board={board.id} aria-label="{board.title} sprints" class="flex min-w-0 flex-col gap-4">
              <h2 class="m-0 font-heading text-base text-ink">{board.title}</h2>

              <div class="flex flex-wrap gap-x-10 gap-y-1">
                {@render projectRow('board', 'This board', board.projectBoard)}
                {@render projectRow('yours', 'Your sessions', board.projectYours)}
              </div>

              {#if rows.length === 0}
                <p class="m-0 text-sm text-ink-mute">This board has no sprints yet.</p>
              {:else}
                <div class="overflow-x-auto">
                  <table class="min-w-[64rem] border-collapse">
                    <thead>
                      <tr>
                        <th scope="col" rowspan="2" class={th}>Sprint</th>
                        <th scope="col" rowspan="2" class={th}>Dates</th>
                        {#each rows[0].figures as f (f.key)}
                          <th scope="col" rowspan="2" class={th}>{f.label}</th>
                        {/each}
                        <th scope="colgroup" colspan="3" class="{th} border-b border-paper-edge">Your sessions</th>
                      </tr>
                      <tr>
                        <th scope="col" class={th}>active AI time</th>
                        <th scope="col" class={th}>fresh tokens</th>
                        <th scope="col" class={th}>API cost</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each rows as row (row.id)}
                        <tr data-sprint={row.id} class="border-t border-paper-edge {board.selectedId === row.id ? 'bg-paper-mute' : ''}">
                          <th scope="row" class="{td} text-left font-normal">
                            <button
                              type="button"
                              data-action="select"
                              aria-pressed={board.selectedId === row.id}
                              class="bg-transparent p-0 text-left text-ink underline"
                              onclick={() => board.select(row.id)}>{row.name}</button
                            >
                            <span class="block text-xs text-ink-soft">{row.state}</span>
                          </th>
                          <td class="{td} text-ink-soft">{row.dates}</td>
                          {#each row.figures as f (f.key)}
                            {@render figureCell(f)}
                          {/each}
                          {#if row.effort.kind === 'cells'}
                            <td class="{td} text-ink" data-effort="active">{row.effort.active}</td>
                            <td class="{td} text-ink" data-effort="fresh">{row.effort.fresh}</td>
                            <td class="{td} text-ink" data-effort="cost">{row.effort.cost}</td>
                          {:else}
                            <td colspan="3" class="{td} text-ink-mute" data-effort="reason">{row.effort.text}</td>
                          {/if}
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}

              {#if board.selected}
                {@const s = board.selected}
                <div data-selected={s.id} class="flex flex-col gap-3 rounded-lg border border-paper-edge bg-paper-soft p-4">
                  <h3 class="m-0 text-sm text-ink">{s.name}</h3>
                  {#if board.facts}
                    <dl class="m-0 flex flex-wrap gap-x-6 gap-y-1 text-sm">
                      {#each board.facts as f (f.key)}
                        <div class="flex gap-1">
                          <dt class="text-ink-mute">{f.label}</dt>
                          <dd class="m-0 text-ink" data-fact={f.key}>{f.value}</dd>
                        </div>
                      {/each}
                    </dl>
                  {/if}
                  {#if board.factsNote}
                    <p class="m-0 text-sm text-ink-soft">{board.factsNote}</p>
                  {/if}
                  {#if board.ticketsError}
                    <p role="alert" class="m-0 text-sm text-danger">{board.ticketsError}</p>
                  {:else if board.tickets === null}
                    <p class="m-0 text-sm text-ink-mute">Reading tickets…</p>
                  {:else if board.tickets.length === 0}
                    <p class="m-0 text-sm text-ink-mute">This sprint lists no tickets.</p>
                  {:else}
                    <ul class="m-0 flex list-none flex-col gap-1 p-0" aria-label="{s.name} tickets">
                      {#each board.tickets as t (t.key)}
                        <li>
                          <a href={t.href} data-ticket={t.key} class="flex gap-3 text-sm text-ink hover:underline">
                            <span class="font-mono text-xs text-ink-soft">{t.key}</span>
                            <span class="truncate">{t.title}</span>
                          </a>
                        </li>
                      {/each}
                    </ul>
                  {/if}
                </div>
              {/if}
            </section>
          {/each}
        </div>
      {/if}
    </div>
  </section>
</Screen>
