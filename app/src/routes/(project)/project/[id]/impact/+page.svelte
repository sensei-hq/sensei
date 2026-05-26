<script lang="ts">
  let { data } = $props();
  let selectedId = $state<string | null>(null);
  // Auto-select first verdict on data load
  $effect(() => { if (selectedId === null && data.verdicts.length > 0) selectedId = data.verdicts[0].id; });
  let selected = $derived(data.verdicts.find((v: any) => v.id === selectedId) ?? null);
</script>

<div class="px-6 py-6">
  <h2 class="text-xl font-normal m-0 mb-1">Impact</h2>
  <p class="text-xs text-surface-z6 m-0 mb-5">
    Each accepted recommendation gets a measurement window. FTR before vs after tells you if it worked.
  </p>

  <div class="flex gap-4 text-xl font-bold mb-5">
    <span class="text-success">↑ {data.positiveCount}</span>
    <span class="text-error">↓ {data.negativeCount}</span>
    <span class="opacity-50">? {data.pendingCount}</span>
  </div>

  {#if data.verdicts.length === 0}
    <p class="text-sm text-surface-z6 opacity-50">No accepted recommendations yet.</p>
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
                <span class="ml-auto mono text-xs" class:text-success={delta > 0} class:text-error={delta < 0}>
                  {delta > 0 ? '+' : ''}{delta}%
                </span>
              {/if}
            </div>
            <p class="text-sm m-0 leading-snug">{v.title}</p>
            {#if v.measured_at}
              <span class="text-xs text-surface-z6 mono mt-1 block">
                measured {new Date(v.measured_at).toLocaleDateString()}
              </span>
            {/if}
          </button>
        {/each}
      </div>

      <!-- Detail panel -->
      {#if selected}
        <div class="p-6 bg-surface-z2 border border-surface-z3 rounded-lg">
          <div class="flex items-center gap-3 mb-4">
            <span class="text-xs mono opacity-50">{selected.urgency}</span>
            {#if selected.acted_at}
              <span class="text-xs opacity-50">acted {new Date(selected.acted_at).toLocaleDateString()}</span>
            {/if}
            {#if selected.measured_at}
              <span class="text-xs opacity-50">measured {new Date(selected.measured_at).toLocaleDateString()}</span>
            {/if}
          </div>

          <h3 class="text-lg font-normal m-0 mb-3">{selected.title}</h3>
          <p class="text-sm text-surface-z7 leading-normal m-0 mb-5">{selected.why}</p>

          {#if selected.baseline_ftr != null || selected.current_ftr != null}
            <div class="grid grid-cols-4 gap-px bg-surface-z3 rounded-md overflow-hidden mb-5">
              <div class="bg-surface-z1 p-3 text-center">
                <span class="block text-xs text-surface-z6 mb-1">FTR Before</span>
                <span class="block text-lg font-bold">
                  {selected.baseline_ftr != null ? Math.round(selected.baseline_ftr * 100) + '%' : '—'}
                </span>
              </div>
              <div class="bg-surface-z1 p-3 text-center">
                <span class="block text-xs text-surface-z6 mb-1">FTR After</span>
                <span class="block text-lg font-bold">
                  {selected.current_ftr != null ? Math.round(selected.current_ftr * 100) + '%' : '—'}
                </span>
              </div>
              <div class="bg-surface-z1 p-3 text-center">
                <span class="block text-xs text-surface-z6 mb-1">Delta</span>
                {#if selected.baseline_ftr != null && selected.current_ftr != null}
                  {@const d = Math.round((selected.current_ftr - selected.baseline_ftr) * 100)}
                  <span class="block text-lg font-bold" class:text-success={d > 0} class:text-error={d < 0}>
                    {d > 0 ? '+' : ''}{d}%
                  </span>
                {:else}
                  <span class="block text-lg opacity-30">—</span>
                {/if}
              </div>
              <div class="bg-surface-z1 p-3 text-center">
                <span class="block text-xs text-surface-z6 mb-1">Verdict</span>
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
              <span class="w-1.5 h-1.5 rounded-full bg-primary-z5"></span>
              <span class="text-primary-z5">{selected.impact}</span>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .verdict-item:hover { background: oklch(var(--color-surface-z2) / 1); }
  .verdict-item.selected { background: oklch(var(--color-surface-z2) / 1); border-left: 2px solid oklch(var(--color-primary-z5) / 1); }
  .text-success { color: oklch(var(--color-success-z5) / 1); }
  .text-error { color: oklch(var(--color-primary-z5) / 1); }
</style>
