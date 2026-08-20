<script lang="ts">
  // Observatory · Sessions — the digest (Slot 6). A retro hero on top, then
  // one quiet chart the user flips between four treatments, then the real
  // captured-session list. All derivations live in ./sessions-digest.ts; this
  // component is pure wiring + layout.
  //
  // Spec:   docs/spec/screen/observatory-sessions.md
  // Mockup: docs/mockups/Sensei/lib/sessions-zen.jsx → SessionsDigestZen
  import { untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Kanji from '$lib/components/Kanji.svelte';
  import {
    TrendChart,
    StreamChart,
    ConstellationChart,
    BandsChart,
    TokensChart,
    MiniChart,
    SessionRow,
  } from '$lib/components/sessions';
  import { SessionsDigestState } from '$lib/sessions-digest.svelte.js';
  import {
    CHART_VARIANTS,
    SESSION_RANGES,
    compactDuration,
    replayHref,
    type ChartVariant,
    type MiniMode,
    type SessionRange,
  } from '$lib/sessions-digest.js';

  let { data } = $props();

  // One state instance per mount, seeded once from the load result; range
  // chips refetch through the api client thereafter. untrack makes the
  // one-time read of `data` explicit (see insights/+page.svelte).
  const digest = new SessionsDigestState(
    (range) =>
      senseiApi(appState.port)
        .getSessionsDigest(range)
        .then((d) => d.sessions),
    untrack(() => data.sessions),
    untrack(() => data.range as SessionRange),
  );

  // Memoized slices so the header and the chart read one filtered source.
  const enriched = $derived(digest.enriched);
  const buckets = $derived(digest.dayBuckets);
  const days = $derived(digest.days);
  const totals = $derived(digest.totals);
  const miniChartMode = $derived(
    (digest.miniMode === 'numbers' ? 'trend' : digest.miniMode) as Exclude<MiniMode, 'numbers'>,
  );
  const medianLabel = $derived(totals.medianMins > 0 ? compactDuration(totals.medianMins) : '—');

  // Chip metadata + cycler glyphs (kanji per mode).
  const CHIP_META: Record<ChartVariant, { kanji: string; sub: string }> = {
    trend: { kanji: '線', sub: 'first-try-right over time' },
    stream: { kanji: '流', sub: 'time spent, stacked' },
    constellation: { kanji: '星', sub: 'duration vs day' },
    bands: { kanji: '帯', sub: 'stacked per day' },
    tokens: { kanji: '燃', sub: 'token volume per day' },
  };
  const MINI_GLYPH: Record<MiniMode, string> = {
    numbers: '数',
    trend: '線',
    stream: '流',
    constellation: '星',
    bands: '帯',
    pulse: '脈',
  };
  const RANGE_LABEL: Record<SessionRange, string> = {
    '7d': '7 days',
    '30d': '30 days',
    '90d': '90 days',
  };

  function selectSession(id: string): void {
    // Deep-link to the Instruments Replay tab; session-id resolves there via
    // GET /api/sessions/{id} (shipped Slot 1).
    goto(replayHref(id));
  }
</script>

<div class="w-full h-full flex flex-col bg-paper overflow-hidden">
  <!-- ── Hero ─────────────────────────────────────────────────────────── -->
  <header class="flex items-center gap-5 pt-5 pb-4 px-6 border-b border-paper-edge shrink-0">
    <Kanji char="録" size="3xl" />
    <div class="flex-1 min-w-0">
      <div class="text-xs uppercase tracking-[0.18em] text-ink-mute mb-1">Observatory · Sessions</div>
      <h1 class="display text-xl font-normal m-0 text-ink">The shape of your week.</h1>
      <p class="text-sm text-ink-soft leading-normal mt-1 mb-0 max-w-[720px]">
        A retrospective. Then one quiet chart — drawn the way you want to read it.
      </p>
      <!-- Range chips — each refetches /api/sessions?range= -->
      <div class="flex items-center gap-2 mt-3" data-testid="range-chips">
        <span class="text-xs uppercase tracking-[0.14em] text-ink-faint">range</span>
        {#each SESSION_RANGES as r (r)}
          {@const on = digest.range === r}
          <button
            type="button"
            class="py-1 px-2 text-xs rounded-full cursor-pointer border"
            class:bg-ink={on}
            class:text-paper={on}
            class:border-ink={on}
            class:bg-transparent={!on}
            class:text-ink-soft={!on}
            class:border-paper-edge={!on}
            data-testid={`range-${r}`}
            aria-pressed={on}
            onclick={() => digest.setRange(r)}
          >
            {RANGE_LABEL[r]}
          </button>
        {/each}
        {#if digest.loading}
          <span class="text-xs text-ink-faint">updating…</span>
        {/if}
      </div>
    </div>

    <!-- Right cluster — numbers at rest, mini chart when cycled; the cycler
         badge steps through numbers/trend/stream/constellation/bands/pulse.
         Pulse lives ONLY on the cycler, never in the chip group below. -->
    <div class="flex items-center gap-3 pl-5 border-l border-paper-edge shrink-0">
      {#if digest.miniMode === 'numbers'}
        <div class="flex gap-4" data-testid="mini-numbers">
          {@render stat('bg-success', totals.good, 'first-try')}
          {@render stat('bg-warning', totals.bad, 'corrected')}
          {@render stat('bg-accent', totals.ugly, 'abandoned')}
          <div class="w-px bg-paper-edge"></div>
          <div class="text-center min-w-[56px]">
            <div class="display text-lg leading-none text-ink" style='font-feature-settings: "tnum";'>{medianLabel}</div>
            <div class="text-xs uppercase tracking-[0.12em] text-ink-faint mt-1">median</div>
          </div>
        </div>
      {:else}
        <div class="flex items-center gap-3" data-testid="mini-view">
          <MiniChart mode={miniChartMode} {buckets} sessions={enriched} {days} />
          <div class="min-w-[56px]">
            <div class="display text-lg leading-none text-ink" style='font-feature-settings: "tnum";'>{totals.count}</div>
            <div class="text-xs uppercase tracking-[0.12em] text-ink-faint mt-1">sessions</div>
          </div>
        </div>
      {/if}
      <button
        type="button"
        class="mini-cycler w-8 h-8 rounded-full bg-paper-soft border border-paper-edge inline-flex items-center justify-center cursor-pointer"
        title={`viewing ${digest.miniMode} · click to cycle`}
        data-testid="mini-cycler"
        data-mode={digest.miniMode}
        onclick={() => digest.cycleMiniMode()}
      >
        <span class="kanji text-sm text-accent leading-none">{MINI_GLYPH[digest.miniMode]}</span>
      </button>
    </div>
  </header>

  <!-- ── Scroll body ──────────────────────────────────────────────────── -->
  <div class="flex-1 overflow-auto min-h-0">
    <!-- Totals strip + quality tally -->
    <div class="flex items-baseline flex-wrap gap-x-4 gap-y-1 pt-4 pb-3 px-6 border-b border-paper-edge">
      <span class="text-sm text-ink">
        <span class="display text-lg" style='font-feature-settings: "tnum";'>{totals.count}</span>
        sessions across {totals.projects} project{totals.projects === 1 ? '' : 's'}
        · median {medianLabel}
      </span>
      <span class="flex-1"></span>
      <span class="text-xs" data-testid="quality-tally">
        <span class="text-success">{totals.good} first-try</span>
        <span class="text-ink-faint"> · </span>
        <span class="text-warning">{totals.bad} corrected</span>
        <span class="text-ink-faint"> · </span>
        <span class="text-accent">{totals.ugly} abandoned</span>
      </span>
    </div>

    <!-- Chart frame — chip group selects the full chart area -->
    <section class="pt-5 pb-4 px-6">
      <div class="flex items-baseline gap-3 mb-3 pb-2 border-b border-paper-edge">
        <span class="kanji text-base text-accent">{CHIP_META[digest.chart].kanji}</span>
        <h2 class="display text-base font-normal m-0 text-ink capitalize">Sessions · {digest.chart}</h2>
        <span class="text-xs text-ink-mute">· {CHIP_META[digest.chart].sub}</span>
        <span class="flex-1"></span>
        <!-- Exactly four chips. pulse is never here (mini-cycler only). -->
        <div class="flex gap-0 p-1 bg-paper-soft border border-paper-edge rounded" data-testid="chart-chips">
          {#each CHART_VARIANTS as v (v)}
            {@const on = digest.chart === v}
            <button
              type="button"
              class="py-1 px-3 text-xs rounded-sm cursor-pointer border capitalize"
              class:bg-paper={on}
              class:text-ink={on}
              class:border-paper-edge={on}
              class:bg-transparent={!on}
              class:text-ink-mute={!on}
              class:border-transparent={!on}
              data-testid={`chip-${v}`}
              aria-pressed={on}
              onclick={() => digest.setChart(v)}
            >
              {v}
            </button>
          {/each}
        </div>
      </div>

      <div class="bg-paper-soft border border-paper-edge rounded-lg pt-5 pb-4 px-5" data-testid="chart-body">
        {#if digest.chart === 'trend'}
          <TrendChart {buckets} />
        {:else if digest.chart === 'stream'}
          <StreamChart {buckets} />
        {:else if digest.chart === 'constellation'}
          <ConstellationChart sessions={enriched} {days} />
        {:else if digest.chart === 'tokens'}
          <TokensChart {buckets} />
        {:else}
          <BandsChart {buckets} />
        {/if}
      </div>

      <!-- Legend — token in/out for the volume chart, quality tones otherwise -->
      <div class="flex justify-end gap-4 mt-3 text-xs text-ink-mute">
        {#if digest.chart === 'tokens'}
          <span class="inline-flex items-center gap-1"><span class="w-2 h-2 rounded-sm bg-accent-soft"></span>input</span>
          <span class="inline-flex items-center gap-1"><span class="w-2 h-2 rounded-sm bg-accent"></span>output</span>
        {:else}
          <span class="inline-flex items-center gap-1"><span class="w-2 h-2 rounded-sm bg-success"></span>first-try</span>
          <span class="inline-flex items-center gap-1"><span class="w-2 h-2 rounded-sm bg-warning"></span>corrected</span>
          <span class="inline-flex items-center gap-1"><span class="w-2 h-2 rounded-sm bg-accent"></span>abandoned</span>
        {/if}
      </div>
    </section>

    <!-- Session list -->
    <section class="pb-8 px-6">
      <div class="flex items-baseline gap-3 mb-2 pb-2 border-b border-paper-edge">
        <span class="kanji text-base text-accent">刻</span>
        <h2 class="display text-base font-normal m-0 text-ink">Sessions</h2>
        <span class="text-xs text-ink-mute">· newest first, click to replay</span>
        <span class="flex-1"></span>
        <span class="font-mono text-xs text-ink-faint">{enriched.length} shown</span>
      </div>
      {#if enriched.length === 0}
        <EmptyState
          kanji="録"
          title="No sessions in this window."
          description="Start a session with your assistant, or widen the range. Each session becomes a moment of learning."
        />
      {:else}
        <div class="flex flex-col gap-px" data-testid="session-list">
          {#each enriched as session (session.id)}
            <SessionRow {session} onselect={selectSession} />
          {/each}
        </div>
      {/if}
    </section>
  </div>
</div>

{#snippet stat(dotClass: string, n: number, label: string)}
  <div class="text-center min-w-[56px]">
    <div class="display text-lg leading-none text-ink inline-flex items-center gap-1" style='font-feature-settings: "tnum";'>
      <span class="w-1.5 h-1.5 rounded-full {dotClass}"></span>
      {n}
    </div>
    <div class="text-xs uppercase tracking-[0.12em] text-ink-faint mt-1">{label}</div>
  </div>
{/snippet}

<style>
  .mini-cycler:hover {
    background: var(--paper);
  }
</style>
