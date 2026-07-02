<script lang="ts">
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import type { ImpactVerdictEntry } from '$lib/types.js';

  let { data } = $props();
  let selectedId = $state<string | null>(null);
  // Auto-select first verdict on data load
  $effect(() => { if (selectedId === null && data.verdicts.length > 0) selectedId = data.verdicts[0].id; });
  let selected = $derived(data.verdicts.find((v: any) => v.id === selectedId) ?? null);

  // Manual impact-log state (T3 Slice 3).
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
  const impactPending = $derived(impactLog.filter(e => e.verdict === 'pending'));
  const impactDecided = $derived(impactLog.filter(e => e.verdict !== 'pending'));
</script>

<div class="px-6 py-6">
  <h2 class="text-xl font-normal m-0 mb-1">Impact</h2>
  <p class="text-xs text-ink-soft m-0 mb-5">
    Each accepted recommendation gets a measurement window. FTR before vs after tells you if it worked.
  </p>

  <div class="flex gap-4 text-xl font-bold mb-5">
    <span class="text-success">↑ {data.positiveCount}</span>
    <span class="text-error">↓ {data.negativeCount}</span>
    <span class="opacity-50">? {data.pendingCount}</span>
  </div>

  {#if data.verdicts.length === 0}
    <p class="text-sm text-ink-soft opacity-50">No accepted recommendations yet.</p>
  {:else}
    <div class="grid grid-cols-[280px_1fr] gap-6 min-h-0">
      <!-- Verdict list -->
      <div class="flex flex-col gap-0.5 overflow-auto">
        {#each data.verdicts as v (v.id)}
          {@const isOpen = selectedId === v.id}
          <button
            class="verdict-item text-left px-3.5 py-3 rounded-md bg-transparent border-none cursor-pointer"
            class:selected={isOpen}
            onclick={() => selectedId = v.id}
          >
            <div class="flex items-center gap-2 mb-1">
              <span class="text-xs font-mono" class:text-success={v.verdict === 'positive'} class:text-error={v.verdict === 'negative'} class:opacity-50={v.verdict === 'pending' || v.verdict === 'neutral'}>
                {v.verdict === 'positive' ? '好' : v.verdict === 'negative' ? '悪' : v.verdict === 'neutral' ? '並' : '?'}
              </span>
              <span class="text-xs uppercase tracking-wide"
                class:text-success={v.verdict === 'positive'}
                class:text-error={v.verdict === 'negative'}
              >{v.verdict}</span>
              {#if v.baseline_ftr != null && v.current_ftr != null}
                {@const delta = Math.round((v.current_ftr - v.baseline_ftr) * 100)}
                <span class="ml-auto font-mono text-xs" class:text-success={delta > 0} class:text-error={delta < 0}>
                  {delta > 0 ? '+' : ''}{delta}%
                </span>
              {/if}
            </div>
            <p class="text-sm m-0 leading-snug">{v.title}</p>
            {#if v.measured_at}
              <span class="text-xs text-ink-soft font-mono mt-1 block">
                measured {new Date(v.measured_at).toLocaleDateString()}
              </span>
            {/if}
          </button>
        {/each}
      </div>

      <!-- Detail panel -->
      {#if selected}
        <div class="p-6 bg-paper-mute border border-paper-mute rounded-lg">
          <div class="flex items-center gap-3 mb-4">
            <span class="text-xs font-mono opacity-50">{selected.urgency}</span>
            {#if selected.acted_at}
              <span class="text-xs opacity-50">acted {new Date(selected.acted_at).toLocaleDateString()}</span>
            {/if}
            {#if selected.measured_at}
              <span class="text-xs opacity-50">measured {new Date(selected.measured_at).toLocaleDateString()}</span>
            {/if}
          </div>

          <h3 class="text-lg font-normal m-0 mb-3">{selected.title}</h3>
          <p class="text-sm text-ink-mute leading-normal m-0 mb-5">{selected.why}</p>

          {#if selected.baseline_ftr != null || selected.current_ftr != null}
            <div class="grid grid-cols-4 gap-px bg-paper-mute rounded-md overflow-hidden mb-5">
              <div class="bg-paper-soft p-3 text-center">
                <span class="block text-xs text-ink-soft mb-1">FTR Before</span>
                <span class="block text-lg font-bold">
                  {selected.baseline_ftr != null ? Math.round(selected.baseline_ftr * 100) + '%' : '—'}
                </span>
              </div>
              <div class="bg-paper-soft p-3 text-center">
                <span class="block text-xs text-ink-soft mb-1">FTR After</span>
                <span class="block text-lg font-bold">
                  {selected.current_ftr != null ? Math.round(selected.current_ftr * 100) + '%' : '—'}
                </span>
              </div>
              <div class="bg-paper-soft p-3 text-center">
                <span class="block text-xs text-ink-soft mb-1">Delta</span>
                {#if selected.baseline_ftr != null && selected.current_ftr != null}
                  {@const d = Math.round((selected.current_ftr - selected.baseline_ftr) * 100)}
                  <span class="block text-lg font-bold" class:text-success={d > 0} class:text-error={d < 0}>
                    {d > 0 ? '+' : ''}{d}%
                  </span>
                {:else}
                  <span class="block text-lg opacity-30">—</span>
                {/if}
              </div>
              <div class="bg-paper-soft p-3 text-center">
                <span class="block text-xs text-ink-soft mb-1">Verdict</span>
                <span class="block text-lg font-bold"
                  class:text-success={selected.verdict === 'positive'}
                  class:text-error={selected.verdict === 'negative'}
                >
                  {selected.verdict}
                </span>
              </div>
            </div>
          {/if}

          {#if selected.impact}
            <div class="flex items-center gap-2 text-xs mb-3">
              <span class="w-1.5 h-1.5 rounded-full bg-accent"></span>
              <span class="text-accent">{selected.impact}</span>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  <!-- ── Manual impact log (T3 Slice 3) ─────────────────────────────── -->
  <section class="mt-10 pt-6 border-t border-paper-mute">
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
      <button
        type="submit"
        disabled={!logForm.title.trim() || logging}
        class="self-start px-4 py-2 rounded-md text-sm bg-primary text-on-primary border-none cursor-pointer"
        data-testid="impact-log-button"
      >{logging ? 'Logging…' : 'Log impact'}</button>
    </form>

    {#if impactPending.length > 0}
      <h4 class="text-xs uppercase tracking-wide opacity-60 m-0 mb-2">Pending verdict</h4>
      <ul class="list-none m-0 p-0 flex flex-col gap-2 mb-6" data-testid="impact-pending-list">
        {#each impactPending as entry (entry.id)}
          <li class="border border-paper-mute rounded-md px-3 py-2 flex items-center gap-2" data-testid={`impact-pending-${entry.id}`}>
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
      <h4 class="text-xs uppercase tracking-wide opacity-60 m-0 mb-2">Decided</h4>
      <ul class="list-none m-0 p-0 flex flex-col gap-1">
        {#each impactDecided as entry (entry.id)}
          <li class="flex items-center gap-2 py-1.5 border-b border-paper-mute text-sm">
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
      <p class="text-sm text-ink-soft opacity-60">No impact entries yet.</p>
    {/if}
  </section>
</div>

<style>
  .verdict-item:hover { background: var(--paper-mute); }
  .verdict-item.selected { background: var(--paper-mute); border-left: 2px solid var(--accent); }
  .text-success { color: var(--success); }
  .text-error { color: var(--accent); }
</style>
