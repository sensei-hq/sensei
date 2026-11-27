<script lang="ts">
  import { Notice, StateView } from '@sensei/ui';
    import Kanji from '$lib/components/Kanji.svelte';
    import { railIcon } from "../observatory-nav";
  import type { Snippet } from 'svelte';
  import { untrack } from 'svelte';
  import { ShellScreen } from '@sensei/ui';
  import type { ShellStat } from '@rokkit/app';
  import { senseiApi } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';
  import { openWorkspaceWindow } from '$lib/stores/windows.svelte.js';
  import { InsightsBoardState, columnToneClass, type RecCardVm } from './insights-board.svelte.js';
  import WorkspaceFilterStrip from './WorkspaceFilterStrip.svelte';
  import RecCard from './RecCard.svelte';
  import ViolationCard from './ViolationCard.svelte';
  import CorrectionMini from './CorrectionMini.svelte';
  import PatternMini from './PatternMini.svelte';
  import MemoryRow from './MemoryRow.svelte';

  let { data } = $props();

  const api = senseiApi(appState.port);
  // Seed the state once from the loaded board; the state owns every refetch
  // and optimistic update thereafter. untrack makes the one-time read explicit.
  const board = new InsightsBoardState(untrack(() => data.board), untrack(() => data.error));

  function selectWorkspace(id: string | null): void {
    void board.selectWorkspace(api, id);
  }

  // The header's now/soon/settled counts, as the shell's stats band.
  const cards = $derived<ShellStat[]>([
    { id: 'now', title: 'Now', value: board.counts.now, tone: 'accent' as const },
    { id: 'soon', title: 'Soon', value: board.counts.soon, tone: 'warning' as const },
    { id: 'settled', title: 'Settled', value: board.counts.settled, tone: 'success' as const },
  ]);
  function apply(rec: RecCardVm): void {
    void board.apply(api, rec);
  }
  function dismiss(rec: RecCardVm): void {
    void board.dismiss(api, rec);
  }
  // Review — navigate to where the recommendation lives (the workspace window).
  // No write call.
  function review(rec: RecCardVm): void {
    openWorkspaceWindow(rec.workspaceId, rec.workspaceName).catch((e) => console.error(e));
  }
</script>

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
      <Kanji
        char={kanji}
        size="xl"
        tone="inherit"
        class={columnToneClass(tone)}
      />
      <div class="flex-1 min-w-0">
        <div class="text-sm text-ink font-medium">{title}</div>
        <div class="text-xs text-ink-mute mt-0.5">{sub}</div>
      </div>
      <span
        class="font-mono text-xs {columnToneClass(tone)}"
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

<!-- Three columns that each scroll, under a fixed header (#293). -->
{#snippet filterBand()}
  <div class="py-3 px-6 shrink-0">
    <WorkspaceFilterStrip
      workspaces={board.workspaces}
      selectedId={board.selectedWorkspaceId}
      onSelect={selectWorkspace}
    />
  </div>
{/snippet}

<!-- On the ScreenShell (D87 §9): the header's mark/title/eyebrow/description
     are the shell's, the counts are its stats band, the workspace strip is
     the filters band, the action notice moves to the body top, the columns
     move in unchanged. -->
<ShellScreen
  viewId="observatory.insights"
  mark={{ kanji: '今', icon: railIcon('/insights') }}
  title="What needs you, what's working, what's quiet."
  context="Observatory · Insights"
  description="Sorted by immediacy. Most days you only need the first column."
  search={false}
  stats={cards}
  filters={filterBand}
>
  {#if board.actionError}
    {#if board.actionNotice}
      <div class="py-2 px-6 border-b border-paper-edge shrink-0">
        <Notice size="inline" expanded {...board.actionNotice} data-insights-action-error />
      </div>
    {/if}
  {/if}
  {#if board.loadError}
    <!-- The board could not be read (#288) — not three quiet columns. Retry
         re-reads the current scope through the state, which owns the board. -->
    <div class="p-8">
      <StateView status="error" error={board.loadError} onretry={() => void board.retry(api)} />
    </div>
  {:else}
  <div class="grid grid-cols-3 flex-1 min-h-0" data-triage-grid>
    {@render columnShell('now', '今', 'Now', 'violations · high-impact · this week', board.now.count, nowBody)}
    {@render columnShell('soon', '近', 'Soon', 'emerging patterns · worth a look', board.soon.count, soonBody)}
    {@render columnShell('settled', '定', 'Settled', 'battle-tested · low-noise reference', board.settled.count, settledBody)}
  </div>
  {/if}
</ShellScreen>
