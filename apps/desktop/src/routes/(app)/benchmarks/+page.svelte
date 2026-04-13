<script lang="ts">
  import { onMount } from 'svelte';

  type BenchTab = 'corpus' | 'runs' | 'results';
  let activeTab = $state<BenchTab>('runs');

  // Placeholder state - will be wired to Tauri/daemon in full implementation
  let runs = $state<Array<{ id: string; repo: string; acp: string; timestamp: string; status: string; bareCost?: number; skillsCost?: number; indexedCost?: number }>>([]);
  let loading = $state(true);

  async function loadRuns() {
    // TODO: Load from .sensei/benchmarks/runs/ via Tauri
    // For now show empty state with instructions
    loading = false;
  }

  onMount(() => { loadRuns(); });
</script>

<div class="flex-1 overflow-y-auto px-6 py-5 space-y-5">

  <div class="flex items-center justify-between">
    <h2 class="text-lg font-semibold text-surface-z8">Benchmarks</h2>
    <button
      class="rounded-lg bg-primary-z3 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z4"
    >
      + New Run
    </button>
  </div>

  <!-- Tab bar -->
  <div class="flex gap-1 border-b border-surface-z0/30 pb-px">
    {#each [['corpus', 'Corpus'], ['runs', 'Runs'], ['results', 'Results']] as [id, label]}
      <button
        onclick={() => activeTab = id as BenchTab}
        class="px-3 py-1.5 text-xs font-medium rounded-t-lg transition-colors
               {activeTab === id ? 'bg-surface-z2 text-surface-z8 border-b-2 border-primary-z5' : 'text-surface-z4 hover:text-surface-z6'}"
      >{label}</button>
    {/each}
  </div>

  {#if activeTab === 'corpus'}
    <div class="rounded-lg bg-surface-z2 p-6 text-center space-y-3">
      <div class="text-3xl">
        <span class="i-solar-document-text-bold-duotone text-surface-z4"></span>
      </div>
      <h3 class="text-sm font-medium text-surface-z7">Benchmark Corpus</h3>
      <p class="text-xs text-surface-z4 max-w-md mx-auto">
        Define benchmark tasks in your repo's <code class="bg-surface-z3 px-1 rounded">.sensei/benchmarks/</code> directory.
        Each task specifies a feature to implement, and sensei measures cost, quality, and speed across bare, skills, and indexed modes.
      </p>
      <button class="rounded-lg bg-primary-z3 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z4">
        Populate Sample Corpus
      </button>
    </div>

  {:else if activeTab === 'runs'}
    {#if loading}
      <p class="text-xs text-surface-z4 py-8 text-center">Loading runs...</p>
    {:else if runs.length === 0}
      <div class="rounded-lg bg-surface-z2 p-6 text-center space-y-3">
        <div class="text-3xl">
          <span class="i-solar-chart-2-bold-duotone text-surface-z4"></span>
        </div>
        <h3 class="text-sm font-medium text-surface-z7">No benchmark runs yet</h3>
        <p class="text-xs text-surface-z4 max-w-md mx-auto">
          Run a benchmark to compare how sensei's indexed context improves AI coding quality.
          Each run tests the same tasks with and without sensei to measure the difference.
        </p>
        <button class="rounded-lg bg-primary-z3 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z4">
          Start First Benchmark
        </button>
      </div>
    {:else}
      <div class="space-y-2">
        {#each runs as run}
          <div class="rounded-lg bg-surface-z2 px-4 py-3 flex items-center gap-4">
            <span class="text-sm font-medium text-surface-z7 flex-1">{run.repo}</span>
            <span class="text-[10px] text-surface-z4">{run.acp}</span>
            <span class="text-[10px] text-surface-z4">{run.timestamp}</span>
            <span class="rounded px-1.5 py-0.5 text-[10px] {run.status === 'completed' ? 'bg-success-z2 text-success-z7' : 'bg-warning-z2 text-warning-z7'}">{run.status}</span>
          </div>
        {/each}
      </div>
    {/if}

  {:else if activeTab === 'results'}
    <div class="rounded-lg bg-surface-z2 p-6 text-center space-y-3">
      <div class="text-3xl">
        <span class="i-solar-graph-bold-duotone text-surface-z4"></span>
      </div>
      <h3 class="text-sm font-medium text-surface-z7">Benchmark Results</h3>
      <p class="text-xs text-surface-z4 max-w-md mx-auto">
        After running benchmarks, results will appear here as comparison charts showing cost savings,
        quality improvements, and time reduction from using sensei's indexed context.
      </p>
    </div>
  {/if}

</div>
