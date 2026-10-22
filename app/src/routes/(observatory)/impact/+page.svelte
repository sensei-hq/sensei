<script lang="ts">
    import Kanji from '$lib/components/Kanji.svelte';
  import { invalidateAll } from '$app/navigation';
  import { EmptyState, MoeReasoningPanel, PageHeader, Screen, ScreenState, UnreadWorkspacesNotice } from '$lib/components';
  import { railIcon } from '../observatory-nav';
  import {
    formatDeltaPct,
    pct,
    verdictMeta as meta,
    verdictToneClass as toneClass,
    bucketImpact,
    type ImpactRow,
  } from '$lib/impact.js';

  let { data } = $props();

  // No buckets means the read failed (#288): the page shows that error.
  const buckets = $derived(data.buckets ?? bucketImpact([]));

  // Flatten buckets into a single ordered feed for the aside list.
  // Order — negative first (loudest to act on), then positive, neutral,
  // pending. Within each bucket the buckets already sort by |ftrDelta|.
  const flat = $derived<ImpactRow[]>([
    ...buckets.negative,
    ...buckets.positive,
    ...buckets.neutral,
    ...buckets.pending,
  ]);

  // The picked-row id — null until the user clicks a row, at which point
  // it's honoured. The `open` derived falls back to the first row when
  // `openId` is null OR the picked row is no longer in the flat list.
  let openId = $state<string | null>(null);
  const open = $derived(
    flat.find((r) => r.id === openId) ?? flat[0],
  );
  const openVm = $derived(open ? meta(open.verdict) : null);
</script>

<!-- List beside detail: each column scrolls itself, under a fixed header (#293). -->
<Screen fill data-impact-total={data.error ? undefined : data.total}>
  {#snippet header()}
    <PageHeader
      kanji="果"
      icon={railIcon('/impact')}
      eyebrow="Observatory · Change impact"
      title="Did sensei's advice actually work?"
      description="Each accepted recommendation gets a measurement window. FTR baseline vs current is compared. The MOE panel writes the reasoning."
    >
      {#snippet right()}
        {#if !data.error}
          <div class="flex gap-5 pl-5 border-l border-paper-edge">
            <div class="text-center">
              <div class="font-mono text-lg font-light text-success">{buckets.positive.length}</div>
              <div class="text-xs uppercase tracking-wider text-ink-faint mt-1">positive</div>
            </div>
            <div class="text-center">
              <div class="font-mono text-lg font-light text-ink">{buckets.neutral.length}</div>
              <div class="text-xs uppercase tracking-wider text-ink-faint mt-1">neutral</div>
            </div>
            <div class="text-center">
              <div class="font-mono text-lg font-light text-warning">{buckets.negative.length}</div>
              <div class="text-xs uppercase tracking-wider text-ink-faint mt-1">negative</div>
            </div>
            <div class="text-center">
              <div class="font-mono text-lg font-light text-ink-soft">{buckets.pending.length}</div>
              <div class="text-xs uppercase tracking-wider text-ink-faint mt-1">pending</div>
            </div>
          </div>
        {/if}
      {/snippet}
    </PageHeader>
  {/snippet}
  {#if data.error}
    <div class="p-8">
      <ScreenState status="error" error={data.error} onretry={() => invalidateAll()} />
    </div>
  {:else}
  {#if data.unread.length > 0}
    <div class="px-6 pt-4"><UnreadWorkspacesNotice unread={data.unread} /></div>
  {/if}
  {#if data.total === 0 || !open}
    <div class="p-6">
      <EmptyState
        kanji="果"
        title="no measurements yet"
        description="Accepted recommendations show up here once the analyzer's MeasureVerdicts pass has enough post-acceptance sessions (typically ≥3 sessions across ≥3 days)."
      />
    </div>
  {:else}
    <!-- Two columns: aside list + main detail -->
    <div class="grid grid-cols-[300px_1fr] min-h-0 flex-1" data-impact-body>
      <aside class="border-r border-paper-edge overflow-auto py-2">
        {#each flat as r (r.id)}
          {@const vm = meta(r.verdict)}
          {@const isOpen = open.id === r.id}
          <button
            type="button"
            class="w-full text-left py-3 px-4 cursor-pointer bg-transparent border-none text-inherit block"
            class:bg-paper-mute={isOpen}
            data-impact-row={r.id}
            data-impact-bucket={r.verdict}
            style={isOpen ? `border-left: 2px solid var(${vm.tone === 'success' ? '--success' : vm.tone === 'warning' ? '--warning' : '--ink-mute'});` : 'border-left: 2px solid transparent;'}
            onclick={() => (openId = r.id)}
          >
            <div class="flex items-center gap-2">
              <Kanji char={vm.glyph} size="inherit" tone="inherit" class="text-sm {toneClass(vm.tone)}" />
              <span class="text-xs uppercase tracking-wider {toneClass(vm.tone)}">{r.verdict}</span>
              <span class="flex-1"></span>
              <span
                class="font-mono text-xs"
                class:text-success={r.ftrDelta != null && r.ftrDelta > 0}
                class:text-warning={r.ftrDelta != null && r.ftrDelta < 0}
                class:text-ink-mute={r.ftrDelta == null || r.ftrDelta === 0}
                data-ftr-delta
              >{formatDeltaPct(r.ftrDelta)}</span>
            </div>
            <div class="text-sm mt-1 leading-snug font-medium truncate"
                 class:text-ink={isOpen}
                 class:text-ink-mute={!isOpen}>{r.title}</div>
            <div class="font-mono text-xs text-ink-faint mt-1 truncate">{r.workspaceName}</div>
          </button>
        {/each}
      </aside>

      <!-- tabindex so keyboard users can scroll the detail pane (a11y:
           scrollable-region-focusable — axe requires it; the Svelte heuristic
           that bans tabindex on non-interactive elements is wrong for scroll regions). -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <main class="overflow-auto pt-5 pb-6 px-7" tabindex="0">
        <!-- Eyebrow line — workspace · verdict metadata -->
        <div class="flex items-center gap-3 text-xs uppercase tracking-wider text-ink-soft mb-3">
          <span class="font-mono normal-case tracking-normal">{open.workspaceName}</span>
          <span class="text-ink-faint">·</span>
          <span>{open.status}</span>
        </div>

        <h2 class="display text-2xl font-light mt-0 mb-4 leading-tight tracking-tight text-ink">
          {open.title}
        </h2>

        <!-- Verdict pill -->
        {#if openVm}
          <div class="flex items-center gap-3 mb-5">
            <!-- Outline chip on a paper surface (which flips for dark mode) + a
                 tone-coloured border and text. A `*-soft` fill can't be used here:
                 the status softs are single-pole (stay pale ~0.94L in BOTH modes),
                 so tone-coloured text on them is near-invisible in dark mode. Paper
                 flips dark, keeping the coloured text high-contrast either way. -->
            <div
              class="inline-flex items-center gap-2 py-1 px-3 rounded-full border bg-paper-soft"
              class:border-success={openVm.tone === 'success'}
              class:border-warning={openVm.tone === 'warning'}
              class:border-paper-edge={openVm.tone === 'ink'}
            >
              <Kanji char={openVm.glyph} size="inherit" tone="inherit" class="text-sm {toneClass(openVm.tone)}" />
              <span class="text-xs uppercase tracking-wider font-medium {toneClass(openVm.tone)}">
                {openVm.label}
              </span>
            </div>
          </div>
        {/if}

        <!-- Before / after FTR strip -->
        <div class="grid grid-cols-4 gap-[1px] bg-paper-edge rounded overflow-hidden mb-5">
          <div class="bg-paper-mute py-3 px-4">
            <div class="text-xs uppercase tracking-wider text-ink-faint mb-1">First-Try-Right</div>
            <div class="font-mono text-sm text-ink-mute">{pct(open.baselineFtr)}</div>
            <div class="font-mono text-lg text-ink">{pct(open.currentFtr)}</div>
            <div
              class="font-mono text-xs mt-1"
              class:text-success={open.ftrDelta != null && open.ftrDelta > 0}
              class:text-warning={open.ftrDelta != null && open.ftrDelta < 0}
              class:text-ink-mute={open.ftrDelta == null || open.ftrDelta === 0}
            >
              {formatDeltaPct(open.ftrDelta)}
            </div>
          </div>
          <div class="bg-paper-mute py-3 px-4">
            <div class="text-xs uppercase tracking-wider text-ink-faint mb-1">Status</div>
            <div class="font-mono text-sm text-ink mt-3">{open.status}</div>
          </div>
          <div class="bg-paper-mute py-3 px-4">
            <div class="text-xs uppercase tracking-wider text-ink-faint mb-1">Verdict</div>
            <div class="font-mono text-sm mt-3 {toneClass(openVm?.tone ?? 'ink')}">{open.verdict}</div>
          </div>
          <div class="bg-paper-mute py-3 px-4">
            <div class="text-xs uppercase tracking-wider text-ink-faint mb-1">Workspace</div>
            <div class="font-mono text-sm text-ink mt-3 truncate">{open.workspaceName}</div>
          </div>
        </div>

        <!-- MOE reasoning — full panel shape when the daemon carries one -->
        {#if open.reasoning}
          <div class="mb-5">
            <MoeReasoningPanel reasoning={open.reasoning} tone={openVm?.tone ?? 'ink'} />
          </div>
        {:else}
          <p class="text-sm text-ink-soft italic">
            No MOE reasoning trace captured for this verdict yet.
          </p>
        {/if}
      </main>
    </div>
  {/if}
  {/if}
</Screen>
