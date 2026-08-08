<script lang="ts">
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import { EmptyState, MoeReasoningPanel } from '$lib/components';
  import { Button } from '@rokkit/ui';
  import {
    formatDeltaPct,
    pct,
    verdictMeta,
    verdictToneClass as toneClass,
    type ImpactRow,
  } from '$lib/impact.js';
  import type { ImpactVerdictEntry } from '$lib/types.js';

  let { data } = $props();

  // Measured verdicts — negative first (loudest to act on), then positive,
  // neutral, pending. Each bucket already sorts by |ftrDelta|.
  const flat = $derived<ImpactRow[]>([
    ...data.buckets.negative,
    ...data.buckets.positive,
    ...data.buckets.neutral,
    ...data.buckets.pending,
  ]);

  // Picked row — null until the user clicks, then honoured; falls back to the
  // first row when null or the picked row drops out of the list.
  let selectedId = $state<string | null>(null);
  const selected = $derived(flat.find((v) => v.id === selectedId) ?? flat[0]);
  const selectedVm = $derived(selected ? verdictMeta(selected.verdict) : null);

  // ── Manual impact-log lane (T3 Slice 3) — independent, user-logged ──────
  const projectId = $derived(page.params.id ?? '');
  let logForm = $state({ title: '', note: '' });
  let logging = $state(false);
  let decideBusy = $state<Record<string, boolean>>({});

  async function logImpact() {
    if (!projectId || !logForm.title.trim()) return;
    logging = true;
    try {
      await senseiApi(appState.port).createImpactVerdict(
        projectId,
        logForm.title.trim(),
        logForm.note.trim() || undefined,
      );
      logForm = { title: '', note: '' };
      await invalidateAll();
    } finally {
      logging = false;
    }
  }

  async function decide(verdictId: string, outcome: 'success' | 'mixed' | 'failure') {
    if (!projectId) return;
    decideBusy = { ...decideBusy, [verdictId]: true };
    try {
      await senseiApi(appState.port).decideImpactVerdict(projectId, verdictId, outcome);
      await invalidateAll();
    } finally {
      decideBusy = { ...decideBusy, [verdictId]: false };
    }
  }

  const impactLog: ImpactVerdictEntry[] = $derived(data.impactLog);
  const impactPending = $derived(impactLog.filter((e) => e.verdict === 'pending'));
  const impactDecided = $derived(impactLog.filter((e) => e.verdict !== 'pending'));
</script>

<div class="px-6 py-6">
  <h2 class="text-xl font-normal m-0 mb-1">Impact</h2>
  <p class="text-[13px] text-ink-mute m-0 mb-5 max-w-[640px] leading-relaxed">
    Each accepted recommendation gets a measurement window. FTR baseline vs
    current tells you whether it worked. The MOE panel writes the reasoning.
  </p>

  <!-- Verdict counts — measured impact at a glance -->
  <div class="flex gap-6 mb-5" data-impact-total={data.total}>
    <div>
      <span class="font-mono text-[17px] font-light text-success">{data.buckets.positive.length}</span>
      <span class="text-xs uppercase tracking-wider text-ink-faint ml-2">positive</span>
    </div>
    <div>
      <span class="font-mono text-[17px] font-light text-ink">{data.buckets.neutral.length}</span>
      <span class="text-xs uppercase tracking-wider text-ink-faint ml-2">neutral</span>
    </div>
    <div>
      <span class="font-mono text-[17px] font-light text-warning">{data.buckets.negative.length}</span>
      <span class="text-xs uppercase tracking-wider text-ink-faint ml-2">negative</span>
    </div>
    <div>
      <span class="font-mono text-[17px] font-light text-ink-soft">{data.buckets.pending.length}</span>
      <span class="text-xs uppercase tracking-wider text-ink-faint ml-2">pending</span>
    </div>
  </div>

  {#if data.total === 0 || !selected}
    <EmptyState
      kanji="果"
      title="no measurements yet"
      description="Accepted recommendations show up here once the analyzer's MeasureVerdicts pass has enough post-acceptance sessions to compare FTR before and after."
    />
  {:else}
    <div class="grid grid-cols-[280px_1fr] gap-6 items-start">
      <!-- Verdict list -->
      <div class="flex flex-col overflow-auto max-h-[560px]">
        {#each flat as v (v.id)}
          {@const vm = verdictMeta(v.verdict)}
          {@const isOpen = selected.id === v.id}
          <button
            type="button"
            class="w-full text-left py-3 px-3.5 rounded-md cursor-pointer bg-transparent border-none border-l-2 text-inherit block"
            class:bg-paper-mute={isOpen}
            class:border-l-transparent={!isOpen}
            class:border-l-success={isOpen && vm.tone === 'success'}
            class:border-l-warning={isOpen && vm.tone === 'warning'}
            class:border-l-ink-mute={isOpen && vm.tone === 'ink'}
            data-verdict-row={v.id}
            data-verdict-bucket={v.verdict}
            onclick={() => (selectedId = v.id)}
          >
            <div class="flex items-center gap-2 mb-1">
              <span class="kanji text-[13px] {toneClass(vm.tone)}">{vm.glyph}</span>
              <span class="text-xs uppercase tracking-wider {toneClass(vm.tone)}">{v.verdict}</span>
              <span class="flex-1"></span>
              <span
                class="font-mono text-xs"
                class:text-success={v.ftrDelta != null && v.ftrDelta > 0}
                class:text-warning={v.ftrDelta != null && v.ftrDelta < 0}
                class:text-ink-mute={v.ftrDelta == null || v.ftrDelta === 0}
              >{formatDeltaPct(v.ftrDelta)}</span>
            </div>
            <p class="text-[13px] m-0 leading-snug font-medium truncate"
               class:text-ink={isOpen}
               class:text-ink-mute={!isOpen}>{v.title}</p>
          </button>
        {/each}
      </div>

      <!-- Detail panel -->
      <div class="min-w-0">
        <!-- Eyebrow — action verb · status -->
        <div class="flex items-center gap-3 text-xs uppercase tracking-wider text-ink-soft mb-3">
          {#if selected.actionType}
            <span class="font-mono normal-case tracking-normal">{selected.actionType}</span>
            <span class="text-ink-faint">·</span>
          {/if}
          <span>{selected.status}</span>
        </div>

        <h3 class="display text-xl font-normal mt-0 mb-4 leading-tight text-ink">{selected.title}</h3>

        <!-- Verdict pill -->
        {#if selectedVm}
          <div class="flex items-center gap-3 mb-5">
            <div
              class="inline-flex items-center gap-2 py-1 px-3 rounded-full border"
              class:bg-success-soft={selectedVm.tone === 'success'}
              class:border-success={selectedVm.tone === 'success'}
              class:bg-warning-soft={selectedVm.tone === 'warning'}
              class:border-warning={selectedVm.tone === 'warning'}
              class:bg-paper-mute={selectedVm.tone === 'ink'}
              class:border-paper-edge={selectedVm.tone === 'ink'}
            >
              <span class="kanji text-[13px] {toneClass(selectedVm.tone)}">{selectedVm.glyph}</span>
              <span class="text-xs uppercase tracking-wider font-medium {toneClass(selectedVm.tone)}">
                {selectedVm.label}
              </span>
            </div>
          </div>
        {/if}

        <!-- Before / after FTR strip -->
        <div class="grid grid-cols-3 gap-[1px] bg-paper-edge rounded overflow-hidden mb-5">
          <div class="bg-paper-mute py-3 px-4">
            <div class="text-xs uppercase tracking-wider text-ink-faint mb-1">First-Try-Right</div>
            <div class="font-mono text-sm text-ink-mute">{pct(selected.baselineFtr)}</div>
            <div class="font-mono text-lg text-ink">{pct(selected.currentFtr)}</div>
            <div
              class="font-mono text-xs mt-1"
              class:text-success={selected.ftrDelta != null && selected.ftrDelta > 0}
              class:text-warning={selected.ftrDelta != null && selected.ftrDelta < 0}
              class:text-ink-mute={selected.ftrDelta == null || selected.ftrDelta === 0}
            >{formatDeltaPct(selected.ftrDelta)}</div>
          </div>
          <div class="bg-paper-mute py-3 px-4">
            <div class="text-xs uppercase tracking-wider text-ink-faint mb-1">Status</div>
            <div class="font-mono text-sm text-ink mt-3">{selected.status}</div>
          </div>
          <div class="bg-paper-mute py-3 px-4">
            <div class="text-xs uppercase tracking-wider text-ink-faint mb-1">Verdict</div>
            <div class="font-mono text-sm mt-3 {toneClass(selectedVm?.tone ?? 'ink')}">{selected.verdict}</div>
          </div>
        </div>

        <!-- MOE reasoning — the expandable trace, when the daemon carries one -->
        {#if selected.reasoning}
          <MoeReasoningPanel reasoning={selected.reasoning} tone={selectedVm?.tone ?? 'ink'} />
        {:else}
          <p class="text-sm text-ink-soft italic m-0">
            No MOE reasoning trace captured for this verdict yet.
          </p>
        {/if}
      </div>
    </div>
  {/if}

  <!-- ── Manual impact log (T3 Slice 3) ─────────────────────────────── -->
  <section class="mt-10 pt-6 border-t border-paper-edge">
    <h3 class="text-sm font-medium m-0 mb-1">Impact log</h3>
    <p class="text-xs text-ink-soft m-0 mb-4">
      Log a shipped change, then verdict it later once the outcome has had
      time to settle. Independent of the automatic recommendation
      measurement above.
    </p>

    <form
      class="flex flex-col gap-2 mb-6"
      onsubmit={(e) => { e.preventDefault(); logImpact(); }}
    >
      <input
        type="text"
        placeholder="What shipped?"
        bind:value={logForm.title}
        class="px-3 py-2 border border-paper-edge rounded-md bg-paper text-ink text-sm outline-none"
        data-testid="impact-title"
      />
      <textarea
        placeholder="Context (optional)"
        bind:value={logForm.note}
        rows="2"
        class="px-3 py-2 border border-paper-edge rounded-md bg-paper text-ink text-sm outline-none resize-y"
        data-testid="impact-note"
      ></textarea>
      <Button
        type="submit"
        variant="primary"
        size="sm"
        class="self-start"
        disabled={!logForm.title.trim() || logging}
        data-testid="impact-log-button"
      >{logging ? 'Logging…' : 'Log impact'}</Button>
    </form>

    {#if impactPending.length > 0}
      <h4 class="text-xs uppercase tracking-wide text-ink-soft m-0 mb-2">Pending verdict</h4>
      <ul class="list-none m-0 p-0 flex flex-col gap-2 mb-6" data-testid="impact-pending-list">
        {#each impactPending as entry (entry.id)}
          <li class="border border-paper-edge rounded-md px-3 py-2 flex items-center gap-2" data-testid={`impact-pending-${entry.id}`}>
            <div class="flex-1 min-w-0">
              <div class="text-sm text-ink truncate">{entry.title}</div>
              {#if entry.note}
                <div class="text-xs text-ink-soft mt-0.5">{entry.note}</div>
              {/if}
              <div class="text-xs text-ink-soft mt-0.5">{new Date(entry.createdAt).toLocaleDateString()}</div>
            </div>
            <button type="button" disabled={decideBusy[entry.id]} onclick={() => decide(entry.id, 'success')}
                    data-testid={`impact-success-${entry.id}`}
                    class="px-2 py-1 text-xs rounded-md bg-success-soft text-success border-none cursor-pointer">Success</button>
            <button type="button" disabled={decideBusy[entry.id]} onclick={() => decide(entry.id, 'mixed')}
                    data-testid={`impact-mixed-${entry.id}`}
                    class="px-2 py-1 text-xs rounded-md bg-warning-soft text-warning border-none cursor-pointer">Mixed</button>
            <button type="button" disabled={decideBusy[entry.id]} onclick={() => decide(entry.id, 'failure')}
                    data-testid={`impact-failure-${entry.id}`}
                    class="px-2 py-1 text-xs rounded-md bg-danger-soft text-danger border-none cursor-pointer">Failure</button>
          </li>
        {/each}
      </ul>
    {/if}

    {#if impactDecided.length > 0}
      <h4 class="text-xs uppercase tracking-wide text-ink-soft m-0 mb-2">Decided</h4>
      <ul class="list-none m-0 p-0 flex flex-col gap-1">
        {#each impactDecided as entry (entry.id)}
          <li class="flex items-center gap-2 py-1.5 border-b border-paper-edge text-sm">
            <span class="text-xs uppercase tracking-wide font-mono w-16"
                  class:text-success={entry.verdict === 'success'}
                  class:text-warning={entry.verdict === 'mixed'}
                  class:text-danger={entry.verdict === 'failure'}
            >{entry.verdict}</span>
            <span class="flex-1 truncate">{entry.title}</span>
            <span class="text-xs text-ink-soft">
              {entry.decidedAt ? new Date(entry.decidedAt).toLocaleDateString() : ''}
            </span>
          </li>
        {/each}
      </ul>
    {/if}

    {#if impactLog.length === 0}
      <p class="text-sm text-ink-soft">No impact entries yet.</p>
    {/if}
  </section>
</div>
