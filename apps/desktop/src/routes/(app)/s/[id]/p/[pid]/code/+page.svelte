<script lang="ts">
  import { page } from '$app/stores';
  import { projectOverviewDummy } from '$lib/observatory/dummy.js';
  import type { ProjectOverview } from '$lib/observatory/types.js';

  let data: ProjectOverview = projectOverviewDummy($page.params.pid ?? '');

  function severityCls(s: string): string {
    if (s === 'error') return 'border-error-z3/50 bg-error-z2/20';
    if (s === 'warning') return 'border-warning-z3/50 bg-warning-z2/20';
    return 'border-surface-z0/30';
  }

  function copyPrompt(prompt: string) {
    navigator.clipboard.writeText(prompt);
  }
</script>

<div class="h-full overflow-y-auto px-6 py-5 space-y-6">

  <!-- Breadcrumb -->
  <div>
    <div class="flex items-center gap-1.5 text-[10px] text-surface-z4">
      <a href="/s/{$page.params.id}" class="text-primary-z5 hover:text-primary-z6">Solution</a>
      <span>/</span>
      <a href="/s/{$page.params.id}/p/{$page.params.pid}" class="text-primary-z5 hover:text-primary-z6">{data.name}</a>
      <span>/</span>
      <span class="text-surface-z6">Code</span>
    </div>
    <h2 class="mt-1 text-lg font-semibold text-surface-z8">{data.name} — Code Intelligence</h2>
    <p class="text-xs text-surface-z4">{data.symbols.functions} functions &middot; {data.symbols.types} types &middot; {data.edges} edges</p>
  </div>

  <!-- Complexity hotspots -->
  {#if data.complexityHotspots.length > 0}
    <div class="space-y-2">
      <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5">Complexity Hotspots</p>
      {#each data.complexityHotspots as item}
        <div class="flex items-start gap-3 rounded-lg border px-4 py-3 {severityCls(item.action.severity)}">
          <div class="flex-1 min-w-0">
            <p class="text-sm font-medium text-surface-z7">{item.name}</p>
            <p class="text-[10px] text-surface-z4 mt-0.5">{item.file}:{item.line} &middot; complexity {item.complexity}</p>
          </div>
          <button onclick={() => copyPrompt(item.action.prompt)}
            class="shrink-0 rounded-md bg-primary-z2 px-2.5 py-1 text-[10px] font-medium text-primary-z7 hover:bg-primary-z3">
            {item.action.label}
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Dead code -->
  {#if data.deadCodeCandidates.length > 0}
    <div class="space-y-2">
      <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5">Dead Code Candidates</p>
      {#each data.deadCodeCandidates as item}
        <div class="flex items-start gap-3 rounded-lg border px-4 py-3 {severityCls(item.action.severity)}">
          <div class="flex-1 min-w-0">
            <p class="text-sm font-medium text-surface-z7">{item.name}</p>
            <p class="text-[10px] text-surface-z4 mt-0.5">{item.file}:{item.line} &middot; {item.kind} &middot; {item.callerCount} callers</p>
          </div>
          <button onclick={() => copyPrompt(item.action.prompt)}
            class="shrink-0 rounded-md bg-primary-z2 px-2.5 py-1 text-[10px] font-medium text-primary-z7 hover:bg-primary-z3">
            {item.action.label}
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Duplicates -->
  {#if data.duplicates.length > 0}
    <div class="space-y-2">
      <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5">Duplicates</p>
      {#each data.duplicates as group}
        <div class="rounded-lg border px-4 py-3 {severityCls(group.action.severity)}">
          <p class="text-sm font-medium text-surface-z7 font-mono">{group.signature}</p>
          <div class="mt-1.5 space-y-0.5">
            {#each group.instances as inst}
              <p class="text-[10px] text-surface-z4">{inst.file}:{inst.line}</p>
            {/each}
          </div>
          <button onclick={() => copyPrompt(group.action.prompt)}
            class="mt-2 rounded-md bg-primary-z2 px-2.5 py-1 text-[10px] font-medium text-primary-z7 hover:bg-primary-z3">
            {group.action.label}
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Doc drift -->
  {#if data.docDrift.length > 0}
    <div class="space-y-2">
      <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5">Doc Drift</p>
      {#each data.docDrift as item}
        <div class="flex items-start gap-3 rounded-lg border px-4 py-3 {severityCls(item.action.severity)}">
          <div class="flex-1 min-w-0">
            <p class="text-sm text-surface-z7">{item.docPath}</p>
            <p class="text-[10px] text-surface-z4 mt-0.5">references changed: {item.changedTarget}</p>
          </div>
          <button onclick={() => copyPrompt(item.action.prompt)}
            class="shrink-0 rounded-md bg-primary-z2 px-2.5 py-1 text-[10px] font-medium text-primary-z7 hover:bg-primary-z3">
            {item.action.label}
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Graph placeholder -->
  <div class="rounded-lg border border-dashed border-surface-z3 p-8 text-center">
    <p class="text-xs text-surface-z4">Graph visualization — wire to existing GraphCanvas component</p>
  </div>

</div>
