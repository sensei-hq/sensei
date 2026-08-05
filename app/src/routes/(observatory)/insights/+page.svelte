<script lang="ts">
  import type { Snippet } from 'svelte';
  import { untrack } from 'svelte';
  import { PageHeader } from '$lib/components';
  import { senseiApi } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';
  import { openProjectWindow } from '$lib/stores/windows.svelte.js';
  import { InsightsBoardState, type RecCardVm } from './insights-board.svelte.js';
  import ProjectFilterStrip from './ProjectFilterStrip.svelte';
  import RecCard from './RecCard.svelte';
  import ViolationCard from './ViolationCard.svelte';
  import CorrectionMini from './CorrectionMini.svelte';
  import PatternMini from './PatternMini.svelte';
  import MemoryRow from './MemoryRow.svelte';

  let { data } = $props();

  const api = senseiApi(appState.port);
  // Seed the state once from the loaded board; the state owns every refetch
  // and optimistic update thereafter. untrack makes the one-time read explicit.
  const board = new InsightsBoardState(untrack(() => data.board));

  function selectProject(id: string | null): void {
    void board.selectProject(api, id);
  }
  function apply(rec: RecCardVm): void {
    void board.apply(api, rec);
  }
  function dismiss(rec: RecCardVm): void {
    void board.dismiss(api, rec);
  }
  // Review — navigate to where the recommendation lives (the project window).
  // No write call.
  function review(rec: RecCardVm): void {
    openProjectWindow(rec.projectId, rec.projectName).catch((e) => console.error(e));
  }
</script>

{#snippet stat(n: number, label: string, tone: 'now' | 'soon' | 'settled')}
  <div class="text-center">
    <div
      class="font-mono text-lg font-light leading-none"
      class:text-accent={tone === 'now'}
      class:text-warning={tone === 'soon'}
      class:text-success={tone === 'settled'}
    >{n}</div>
    <div class="text-xs uppercase tracking-wider text-ink-faint mt-1">{label}</div>
  </div>
{/snippet}

{#snippet quiet(text: string)}
  <p class="text-xs text-ink-faint italic text-center py-6 m-0">{text}</p>
{/snippet}

{#snippet columnShell(
  tone: 'now' | 'soon' | 'settled',
  kanji: string,
  title: string,
  sub: string,
  count: number,
  body: Snippet,
)}
  <section
    class="flex flex-col min-h-0 border-r border-paper-edge last:border-r-0"
    data-triage-column={tone}
    data-count={count}
  >
    <div class="flex items-baseline gap-2 pt-4 pb-3 px-5 border-b border-paper-edge">
      <span
        class="kanji text-xl leading-none"
        class:text-accent={tone === 'now'}
        class:text-warning={tone === 'soon'}
        class:text-success={tone === 'settled'}
      >{kanji}</span>
      <div class="flex-1 min-w-0">
        <div class="text-sm text-ink font-medium">{title}</div>
        <div class="text-xs text-ink-soft mt-0.5">{sub}</div>
      </div>
      <span
        class="font-mono text-xs"
        class:text-accent={tone === 'now'}
        class:text-warning={tone === 'soon'}
        class:text-success={tone === 'settled'}
      >{count}</span>
    </div>
    <!-- Keyboard-focusable so a keyboard user can scroll the column (a11y:
         scrollable-region-focusable). role/aria-label give it an accessible name.
         The Svelte no-noninteractive-tabindex heuristic is wrong for scroll regions. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      class="flex-1 overflow-auto flex flex-col gap-2 py-3 px-4"
      tabindex="0"
      role="group"
      aria-label={`${title} column`}
    >
      {@render body()}
    </div>
  </section>
{/snippet}

{#snippet nowBody()}
  {#each board.now.violations as m (m.id)}
    <ViolationCard memory={m} />
  {/each}
  {#each board.now.recs as rec, i (rec.id)}
    <RecCard
      {rec}
      focal={i === 0 && board.now.violations.length === 0}
      onApply={() => apply(rec)}
      onReview={() => review(rec)}
      onDismiss={() => dismiss(rec)}
    />
  {/each}
  {#each board.now.corrections as c (c.id)}
    <CorrectionMini correction={c} />
  {/each}
  {#if board.now.count === 0}
    {@render quiet('nothing urgent.')}
  {/if}
{/snippet}

{#snippet soonBody()}
  {#each board.soon.recs as rec (rec.id)}
    <RecCard
      {rec}
      onApply={() => apply(rec)}
      onReview={() => review(rec)}
      onDismiss={() => dismiss(rec)}
    />
  {/each}
  {#each board.soon.patterns as p (p.id)}
    <PatternMini pattern={p} />
  {/each}
  {#each board.soon.memories as m (m.id)}
    <MemoryRow memory={m} variant="proposed" />
  {/each}
  {#if board.soon.count === 0}
    {@render quiet('nothing brewing.')}
  {/if}
{/snippet}

{#snippet settledBody()}
  {#each board.settled.memories as m (m.id)}
    <MemoryRow memory={m} variant="settled" />
  {/each}
  {#each board.settled.patterns as p (p.id)}
    <PatternMini pattern={p} />
  {/each}
  {#if board.settled.count === 0}
    {@render quiet('nothing yet.')}
  {/if}
{/snippet}

<div class="flex flex-col h-full">
  <PageHeader
    variant="h1"
    kanji="今"
    eyebrow="Observatory · Insights"
    title="What needs you, what's working, what's quiet."
    description="Sorted by immediacy. Most days you only need the first column."
  >
    {#snippet right()}
      <div class="flex gap-5 pl-5 border-l border-paper-edge">
        {@render stat(board.counts.now, 'now', 'now')}
        {@render stat(board.counts.soon, 'soon', 'soon')}
        {@render stat(board.counts.settled, 'settled', 'settled')}
      </div>
    {/snippet}
  </PageHeader>

  <div class="py-3 px-6 border-b border-paper-edge shrink-0">
    <ProjectFilterStrip
      projects={board.projects}
      selectedId={board.selectedProjectId}
      onSelect={selectProject}
    />
  </div>

  {#if board.actionError}
    <div
      role="alert"
      class="flex items-center gap-2 py-2 px-6 bg-danger-soft border-b border-paper-edge shrink-0"
    >
      <span class="kanji text-xs text-danger">警</span>
      <span class="text-xs text-danger">{board.actionError}</span>
    </div>
  {/if}

  <div class="grid grid-cols-3 flex-1 min-h-0" data-triage-grid>
    {@render columnShell('now', '今', 'Now', 'violations · high-impact · this week', board.now.count, nowBody)}
    {@render columnShell('soon', '近', 'Soon', 'emerging patterns · worth a look', board.soon.count, soonBody)}
    {@render columnShell('settled', '定', 'Settled', 'battle-tested · low-noise reference', board.settled.count, settledBody)}
  </div>
</div>
