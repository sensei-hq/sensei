<script lang="ts">
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();

  const STATUS_CLS: Record<string, string> = {
    active:   'bg-success-z2 text-success-z7',
    recent:   'bg-primary-z2 text-primary-z7',
    stale:    'bg-warning-z2 text-warning-z7',
    archived: 'bg-surface-z3 text-surface-z5',
    unknown:  'bg-surface-z3 text-surface-z5',
  };

  const SENSEI_API = 'http://localhost:7744';

  type Library = typeof data.libraries[0];
  let selected = $state<Library | null>(null);

  // MCP Explorer state
  type McpTool = { id: string; label: string; desc: string; args: McpArg[] };
  type McpArg = { key: string; label: string; placeholder: string; optional?: boolean };

  const tools: McpTool[] = [
    {
      id: 'search',
      label: 'Search symbols',
      desc: 'Semantic search across indexed symbols and docs',
      args: [{ key: 'query', label: 'Query', placeholder: 'function or concept to search for' }],
    },
    {
      id: 'list_exports',
      label: 'List exports',
      desc: 'Show all exported symbols from the library',
      args: [{ key: 'module', label: 'Module (optional)', placeholder: 'e.g. src/index.ts', optional: true }],
    },
    {
      id: 'get_lib_docs',
      label: 'Get docs',
      desc: 'Retrieve llms.txt documentation for this library',
      args: [{ key: 'section', label: 'Section (optional)', placeholder: 'e.g. API, Usage', optional: true }],
    },
    {
      id: 'check_drift',
      label: 'Check drift',
      desc: 'Detect symbols whose docs are out of sync with code',
      args: [],
    },
  ];

  let activeTool = $state<McpTool>(tools[0]);
  let argValues = $state<Record<string, string>>({});
  let running = $state(false);
  let output = $state<{ ok: boolean; result?: unknown; error?: string } | null>(null);

  $effect(() => {
    // reset when tool changes
    void activeTool;
    argValues = {};
    output = null;
  });

  $effect(() => {
    // reset output when library changes
    void selected;
    output = null;
  });

  async function runTool() {
    if (!selected) return;
    running = true;
    output = null;
    try {
      const args: Record<string, unknown> = { repoPath: selected.path };
      for (const arg of activeTool.args) {
        if (argValues[arg.key]) args[arg.key] = argValues[arg.key];
      }
      const res = await fetch(`${SENSEI_API}/api/mcp`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ tool: activeTool.id, args }),
      });
      output = await res.json() as typeof output;
    } catch (e) {
      output = { ok: false, error: String(e) };
    } finally {
      running = false;
    }
  }

  function formatOutput(result: unknown): string {
    if (typeof result === 'string') return result;
    return JSON.stringify(result, null, 2);
  }
</script>

<div class="flex h-full min-h-0">

  <!-- ══ LIBRARY LIST ══════════════════════════════════════════════════ -->
  <div class="flex w-64 shrink-0 flex-col border-r border-surface-z0/50 overflow-hidden">
    <div class="flex items-center justify-between border-b border-surface-z0/50 px-3 py-2 shrink-0">
      <h1 class="text-sm font-semibold text-surface-z8">Libraries</h1>
      <span class="text-xs text-surface-z4">{data.libraries.length}</span>
    </div>

    <div class="flex-1 overflow-y-auto py-1">
      {#if data.libraries.length === 0}
        <div class="flex flex-col items-center justify-center h-full gap-3 text-center px-4 py-12">
          <span class="i-solar-box-bold-duotone text-2xl text-surface-z3"></span>
          <p class="text-xs text-surface-z4">No libraries yet.<br>Import repos and tag them as <code class="bg-surface-z3 px-0.5 rounded">library</code>.</p>
        </div>
      {:else}
        {#each data.libraries as lib}
          <button
            onclick={() => selected = lib}
            class="flex w-full items-start gap-2.5 px-3 py-2.5 text-left transition-colors hover:bg-surface-z2
                   {selected?.path === lib.path ? 'bg-primary-z1 border-r-2 border-primary-z5' : ''}"
          >
            <span class="i-solar-box-bold-duotone text-sm text-info-z6 shrink-0 mt-0.5"></span>
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-1.5">
                <span class="text-xs font-semibold text-surface-z8 truncate">{lib.name}</span>
                <span class="shrink-0 rounded-full px-1 py-px text-[9px] {STATUS_CLS[lib.status] ?? STATUS_CLS.unknown}">{lib.status}</span>
              </div>
              {#if lib.description}
                <p class="text-[10px] text-surface-z4 line-clamp-1 mt-0.5">{lib.description}</p>
              {/if}
              <div class="flex gap-1 mt-1 flex-wrap">
                {#each lib.tech_stack.slice(0, 2) as t}
                  <span class="text-[9px] bg-surface-z3 text-surface-z5 px-1 rounded">{t}</span>
                {/each}
              </div>
            </div>
          </button>
        {/each}
      {/if}
    </div>
  </div>

  <!-- ══ MCP EXPLORER ════════════════════════════════════════════════ -->
  <div class="flex flex-1 min-w-0 flex-col">
    {#if !selected}
      <div class="flex flex-col items-center justify-center h-full gap-3 text-center text-surface-z4">
        <span class="i-solar-cpu-bold-duotone text-3xl text-surface-z3"></span>
        <div>
          <p class="text-sm font-medium text-surface-z6">Select a library</p>
          <p class="text-xs mt-1">Run MCP tools and see results</p>
        </div>
      </div>
    {:else}
      <!-- Header -->
      <div class="border-b border-surface-z0/50 px-5 py-3 shrink-0">
        <div class="flex items-center gap-2">
          <span class="i-solar-box-bold-duotone text-base text-info-z6"></span>
          <h2 class="text-sm font-semibold text-surface-z8">{selected.name}</h2>
          {#if selected.client}
            <span class="rounded-full bg-surface-z3 px-1.5 py-0.5 text-[10px] text-surface-z5">{selected.client}</span>
          {/if}
        </div>
        {#if selected.description}
          <p class="text-xs text-surface-z4 mt-0.5">{selected.description}</p>
        {/if}
        <p class="text-[10px] font-mono text-surface-z3 mt-0.5 truncate">{selected.path}</p>
      </div>

      <!-- Tool selector + args + run -->
      <div class="border-b border-surface-z2 px-5 py-3 shrink-0 space-y-3">
        <!-- Tool tabs -->
        <div class="flex gap-1 flex-wrap">
          {#each tools as tool}
            <button
              onclick={() => activeTool = tool}
              class="rounded-lg px-2.5 py-1.5 text-xs transition-colors
                     {activeTool.id === tool.id
                       ? 'bg-primary-z6 text-white font-medium'
                       : 'bg-surface-z2 text-surface-z5 hover:bg-surface-z3'}"
            >
              {tool.label}
            </button>
          {/each}
        </div>
        <p class="text-xs text-surface-z4">{activeTool.desc}</p>

        <!-- Args -->
        {#each activeTool.args as arg}
          <div class="flex gap-2 items-center">
            <label class="text-xs text-surface-z5 w-28 shrink-0">{arg.label}</label>
            <input
              bind:value={argValues[arg.key]}
              placeholder={arg.placeholder}
              class="flex-1 rounded-lg border border-surface-z3 bg-surface-z2 px-3 py-1.5 text-sm text-surface-z7 outline-none placeholder:text-surface-z3 focus:border-primary-z4"
            />
          </div>
        {/each}

        <!-- Run button -->
        <button
          onclick={runTool}
          disabled={running}
          class="flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-semibold transition-colors
                 {running ? 'bg-surface-z3 text-surface-z4' : 'bg-primary-z6 text-white hover:bg-primary-z7'}"
        >
          {#if running}
            <span class="i-solar-refresh-bold-duotone animate-spin text-sm"></span>Running…
          {:else}
            <span class="i-solar-play-bold-duotone text-sm"></span>Run
          {/if}
        </button>
      </div>

      <!-- Output -->
      <div class="flex-1 min-h-0 overflow-y-auto px-5 py-4">
        {#if !output}
          <p class="text-xs text-surface-z3 text-center py-8">Run a tool to see output here</p>
        {:else if !output.ok}
          <div class="rounded-xl border border-error-z3 bg-error-z1 px-4 py-3">
            <p class="text-xs font-semibold text-error-z6 mb-1">Error</p>
            <p class="text-xs text-error-z5 font-mono">{output.error}</p>
          </div>
        {:else}
          <pre class="text-xs text-surface-z7 font-mono whitespace-pre-wrap break-words leading-relaxed bg-surface-z2 rounded-xl p-4 border border-surface-z2">{formatOutput(output.result)}</pre>
        {/if}
      </div>
    {/if}
  </div>

</div>
