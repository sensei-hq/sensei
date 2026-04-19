<script lang="ts">
  import { toolsDummy } from '$lib/observatory/dummy.js';
  import type { ToolInfo } from '$lib/observatory/types.js';

  let tools: ToolInfo[] = toolsDummy();
  let search = $state('');
  let filtered = $derived(
    search ? tools.filter(t => t.name.includes(search) || t.description.includes(search)) : tools
  );

  // Simulate tool call
  let simTool = $state<string | null>(null);
  let simParams = $state<Record<string, string>>({});
  let simResult = $state<string | null>(null);
  let simRunning = $state(false);

  function startSim(tool: ToolInfo) {
    simTool = tool.name;
    simParams = Object.fromEntries(tool.params.map(p => [p, '']));
    simResult = null;
  }

  async function runSim() {
    simRunning = true;
    // TODO: call real /api/mcp/call endpoint
    await new Promise(r => setTimeout(r, 500));
    simResult = JSON.stringify({ ok: true, results: [`Mock result for ${simTool}(${JSON.stringify(simParams)})`] }, null, 2);
    simRunning = false;
  }
</script>

<div class="h-full overflow-y-auto px-6 py-5 space-y-5">

  <div class="flex items-center justify-between">
    <h2 class="text-lg font-semibold text-surface-z8">MCP Tools</h2>
    <input
      type="text" bind:value={search} placeholder="Filter tools..."
      class="rounded-md border border-surface-z3 bg-surface-z1 px-2 py-1 text-xs text-surface-z7 outline-none focus:border-primary-z4 w-48"
    />
  </div>

  <div class="grid grid-cols-[1fr_320px] gap-6">

    <!-- Tool list -->
    <div class="space-y-1">
      {#each filtered as tool (tool.name)}
        <button
          onclick={() => startSim(tool)}
          class="flex w-full items-center gap-3 rounded-lg bg-surface-z2 px-3 py-2.5 text-sm text-left hover:bg-surface-z3/60 transition-colors
                 {simTool === tool.name ? 'ring-1 ring-primary-z4' : ''}"
        >
          <span class="flex h-7 w-7 items-center justify-center rounded-md bg-primary-z2 text-[10px] font-bold text-primary-z7">
            {tool.name.slice(0, 2)}
          </span>
          <div class="flex-1 min-w-0">
            <p class="font-medium text-surface-z7">{tool.name}</p>
            <p class="text-[10px] text-surface-z4 truncate">{tool.description}</p>
          </div>
          <div class="text-right shrink-0">
            <p class="text-xs text-surface-z6">{tool.usageCount}x</p>
            {#if tool.errorCount > 0}
              <p class="text-[10px] text-error-z6">{tool.errorCount} errors</p>
            {/if}
          </div>
        </button>
      {/each}
    </div>

    <!-- Simulation panel -->
    <div class="space-y-3">
      {#if simTool}
        <div class="rounded-lg bg-surface-z2/50 border border-surface-z0/30 p-4 space-y-3">
          <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5">Try: {simTool}</p>
          {#each Object.entries(simParams) as [param, value]}
            <div>
              <label class="text-[10px] text-surface-z4 block mb-0.5">{param}</label>
              <input
                type="text" bind:value={simParams[param]}
                class="w-full rounded-md border border-surface-z3 bg-surface-z1 px-2 py-1 text-xs text-surface-z7 outline-none focus:border-primary-z4"
                placeholder={param}
              />
            </div>
          {/each}
          <button
            onclick={runSim}
            disabled={simRunning}
            class="rounded-md bg-primary-z2 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z3 disabled:opacity-50"
          >
            {simRunning ? 'Running...' : 'Execute'}
          </button>
          {#if simResult}
            <div class="rounded-md bg-surface-z1 p-2 text-[10px] font-mono text-surface-z6 overflow-auto max-h-48">
              <pre>{simResult}</pre>
            </div>
          {/if}
        </div>
      {:else}
        <div class="rounded-lg bg-surface-z2/50 border border-surface-z0/30 p-4 text-center">
          <p class="text-xs text-surface-z4">Select a tool to try it</p>
        </div>
      {/if}
    </div>

  </div>

</div>
