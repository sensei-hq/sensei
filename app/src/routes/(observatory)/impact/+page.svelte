<script lang="ts">
  import { PageHeader, EmptyState, Eyebrow } from '$lib/components';
  import { formatDeltaPct, type ImpactRow } from './aggregate.js';

  let { data } = $props();

  // Flatten buckets into a single ordered feed for the aside list.
  // Order — negative first (loudest to act on), then positive, neutral,
  // pending. Within each bucket the buckets already sort by |ftrDelta|.
  const flat = $derived<ImpactRow[]>([
    ...data.buckets.negative,
    ...data.buckets.positive,
    ...data.buckets.neutral,
    ...data.buckets.pending,
  ]);

  // The picked-row id — null until the user clicks a row, at which point
  // it's honoured. The `open` derived falls back to the first row when
  // `openId` is null OR the picked row is no longer in the flat list.
  let openId = $state<string | null>(null);
  const open = $derived(
    flat.find((r) => r.id === openId) ?? flat[0],
  );
  const openVm = $derived(open ? meta(open.verdict) : null);

  const VERDICT_META = {
    positive: { glyph: '好', tone: 'success', label: 'positive impact' },
    neutral:  { glyph: '並', tone: 'ink',     label: 'no measurable effect' },
    negative: { glyph: '悪', tone: 'warning', label: 'negative impact' },
    pending:  { glyph: '待', tone: 'ink',     label: 'measurement pending' },
  } as const;

  function meta(v: ImpactRow['verdict']) {
    return VERDICT_META[v] ?? VERDICT_META.pending;
  }

  function pct(v: number | null): string {
    if (v == null) return '—';
    return `${Math.round(v * 100)}%`;
  }

  function toneClass(tone: string, kind: 'text' | 'bg' | 'border' = 'text'): string {
    const prefix = kind === 'text' ? 'text' : kind === 'bg' ? 'bg' : 'border';
    if (tone === 'success') return `${prefix}-success`;
    if (tone === 'warning') return `${prefix}-warning`;
    return `${prefix}-ink-mute`;
  }
</script>

<div class="flex flex-col h-full" data-impact-total={data.total}>
  <!-- Header — kanji + eyebrow + title + description + 4 verdict counts -->
  <div class="flex items-center gap-5 pt-5 pb-4 px-6 border-b border-paper-edge">
    <span class="kanji text-[40px] text-accent leading-none">果</span>
    <div class="flex-1 min-w-0">
      <p class="m-0 mb-1"><Eyebrow>Observatory · Change impact</Eyebrow></p>
      <h1 class="display text-[22px] font-normal m-0">Did sensei's advice actually work?</h1>
      <p class="text-[13px] text-ink-mute mt-1 mb-0 max-w-[720px] leading-relaxed">
        Each accepted recommendation gets a measurement window. FTR baseline
        vs current is compared. The MOE panel writes the reasoning.
      </p>
    </div>
    <div class="flex gap-5 pl-5 border-l border-paper-edge">
      <div class="text-center">
        <div class="font-mono text-[17px] font-light text-success">{data.buckets.positive.length}</div>
        <div class="text-xs uppercase tracking-wider text-ink-faint mt-1">positive</div>
      </div>
      <div class="text-center">
        <div class="font-mono text-[17px] font-light text-ink">{data.buckets.neutral.length}</div>
        <div class="text-xs uppercase tracking-wider text-ink-faint mt-1">neutral</div>
      </div>
      <div class="text-center">
        <div class="font-mono text-[17px] font-light text-warning">{data.buckets.negative.length}</div>
        <div class="text-xs uppercase tracking-wider text-ink-faint mt-1">negative</div>
      </div>
      <div class="text-center">
        <div class="font-mono text-[17px] font-light text-ink-soft">{data.buckets.pending.length}</div>
        <div class="text-xs uppercase tracking-wider text-ink-faint mt-1">pending</div>
      </div>
    </div>
  </div>

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
              <span class="kanji text-[13px] {toneClass(vm.tone)}">{vm.glyph}</span>
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
            <div class="text-[13px] mt-1 leading-snug font-medium truncate"
                 class:text-ink={isOpen}
                 class:text-ink-mute={!isOpen}>{r.title}</div>
            <div class="font-mono text-xs text-ink-faint mt-1 truncate">{r.projectName}</div>
          </button>
        {/each}
      </aside>

      <main class="overflow-auto pt-5 pb-6 px-7">
        <!-- Eyebrow line — project · verdict metadata -->
        <div class="flex items-center gap-3 text-xs uppercase tracking-wider text-ink-soft mb-3">
          <span class="font-mono normal-case tracking-normal">{open.projectName}</span>
          <span class="text-ink-faint">·</span>
          <span>{open.status}</span>
        </div>

        <h2 class="display text-[28px] font-light mt-0 mb-4 leading-tight tracking-tight text-ink">
          {open.title}
        </h2>

        <!-- Verdict pill -->
        {#if openVm}
          <div class="flex items-center gap-3 mb-5">
            <div
              class="inline-flex items-center gap-2 py-1 px-3 rounded-full border"
              class:bg-success-soft={openVm.tone === 'success'}
              class:border-success={openVm.tone === 'success'}
              class:bg-warning-soft={openVm.tone === 'warning'}
              class:border-warning={openVm.tone === 'warning'}
              class:bg-paper-mute={openVm.tone === 'ink'}
              class:border-paper-edge={openVm.tone === 'ink'}
            >
              <span class="kanji text-[13px] {toneClass(openVm.tone)}">{openVm.glyph}</span>
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
            <div class="text-xs uppercase tracking-wider text-ink-faint mb-1">Project</div>
            <div class="font-mono text-sm text-ink mt-3 truncate">{open.projectName}</div>
          </div>
        </div>

        <!-- MOE reasoning — only when the daemon carries a reasoning trace -->
        {#if open.reasoning}
          {@const models = open.reasoning.models ?? []}
          <div
            class="py-4 px-5 mb-5 rounded bg-paper-mute border border-paper-edge"
            style={`border-left: 2px solid var(${openVm?.tone === 'success' ? '--success' : openVm?.tone === 'warning' ? '--warning' : '--ink-mute'});`}
          >
            <div class="flex items-center gap-2 mb-3">
              <span class="kanji text-[13px] text-accent">議</span>
              <span class="text-xs uppercase tracking-wider text-ink-soft">MOE panel reasoning</span>
              <span class="flex-1"></span>
              {#if open.reasoning.consensus}
                <span class="font-mono text-xs text-ink-soft">{open.reasoning.consensus}</span>
              {/if}
            </div>

            {#if models.length > 0}
              <div class="flex flex-col gap-1 pt-3 border-t border-paper-edge">
                {#each models as name (name)}
                  <div class="grid grid-cols-[120px_14px_1fr] gap-2 items-start">
                    <span class="font-mono text-xs text-ink">{name}</span>
                    <span class="kanji text-[13px] text-accent mt-1">議</span>
                    <span class="text-xs text-ink-mute leading-relaxed">contributed to consensus</span>
                  </div>
                {/each}
              </div>
            {:else}
              <p class="text-xs text-ink-mute italic m-0">
                No per-model breakdown captured for this verdict yet.
              </p>
            {/if}
          </div>
        {:else}
          <p class="text-sm text-ink-soft italic">
            No MOE reasoning trace captured for this verdict yet.
          </p>
        {/if}
      </main>
    </div>
  {/if}
</div>
