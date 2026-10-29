<script lang="ts">
  // The Code map (M17): one canvas, three levels — modules, files, symbols —
  // with a breadcrumb, a grouping, the roles shown, and lenses on the same
  // canvas. The host passes the state and where to read from, and lists the
  // lenses itself (`state.showLens`); this never fetches on its own.
  import { onMount } from 'svelte';
  import { ROLES, type CodeMapState } from './state.svelte';
  import { complexityShare, couplingBands, cycleBands, cycleOf } from './lenses';
  import type { CodeMapPort, GroupBy } from './types';
  import ModuleMap from './ModuleMap.svelte';
  import MatrixLens from './MatrixLens.svelte';
  import ZonesLens from './ZonesLens.svelte';
  import WheelLens from './WheelLens.svelte';
  import ModuleList from './ModuleList.svelte';
  import FileColumns from './FileColumns.svelte';
  import SymbolView from './SymbolView.svelte';

  let { state: map, port }: { state: CodeMapState; port: CodeMapPort } = $props();

  onMount(() => {
    if (!map.l0) void map.load(port);
  });

  // Complexity (M17): tinted by the share of functions over cyclomatic 10;
  // a block never measured stays neutral and says so.
  const complexityFill = (b: import('./types').Block) => {
    const share = complexityShare(b);
    return share === null
      ? { color: 'var(--paper-mute)', opacity: 1, label: 'not measured' }
      : { color: 'var(--danger)', opacity: 0.08 + 0.72 * Math.min(1, share * 4), label: `${Math.round(share * 100)}% over cx 10` };
  };
  const GROUPINGS: { id: GroupBy; label: string }[] = [
    { id: 'layers', label: 'Layers' },
    { id: 'folders', label: 'Folders' },
  ];

  let width = $state(1024);
  // An unmeasured width (0) is not a narrow screen.
  const narrow = $derived(width > 0 && width < 640);
  const l0 = $derived(map.l0);
  const bands = $derived(
    !l0 ? [] : map.lens === 'cycles' ? cycleBands(l0) : map.lens === 'coupling' ? couplingBands(l0, map.cochange?.hidden ?? []) : l0.bands,
  );
  const cycles = $derived(l0 && map.lens === 'cycles' ? cycleOf(l0) : null);
  const picked = $derived(l0?.blocks.find((b) => b.id === map.selected) ?? null);
  const open = (id: string) => void map.openModule(port, id);
</script>

<div class="flex flex-col gap-3" bind:clientWidth={width} data-testid="codemap">
  <div class="flex flex-wrap items-center gap-3">
    <nav class="flex items-center gap-1 text-sm" aria-label="Code map level" data-testid="codemap-crumbs">
      {#each map.crumbs as c, i (i)}
        {#if i > 0}<span class="text-ink-faint">›</span>{/if}
        <button class="bg-transparent border-none cursor-pointer p-0 font-mono text-xs"
          class:text-ink={i === map.crumbs.length - 1} class:text-ink-mute={i < map.crumbs.length - 1}
          onclick={() => map.crumbTo(i)}>{c.label}</button>
      {/each}
      <span class="ml-2 px-2 py-0.5 rounded bg-paper-mute text-xs text-ink-mute" data-testid="codemap-level">
        L{map.crumbs.length - 1} · {map.level}
      </span>
    </nav>
    <div class="flex-1"></div>
    {#if map.level === 'modules'}
      <div class="flex gap-1" role="radiogroup" aria-label="Group by">
        {#each GROUPINGS as g (g.id)}
          <button class="px-2 py-1 text-xs rounded border border-paper-edge cursor-pointer"
            class:bg-ink={map.groupBy === g.id} class:text-paper={map.groupBy === g.id} class:bg-transparent={map.groupBy !== g.id}
            role="radio" aria-checked={map.groupBy === g.id}
            onclick={() => void map.setGroupBy(port, g.id)} data-testid={`codemap-group-${g.id}`}>{g.label}</button>
        {/each}
      </div>
    {:else if map.level === 'files'}
      <div class="flex gap-1" aria-label="Show">
        {#each ROLES as r (r)}
          <button class="px-2 py-1 text-xs rounded border border-paper-edge cursor-pointer"
            class:bg-paper-mute={map.roles.includes(r)} class:bg-transparent={!map.roles.includes(r)}
            aria-pressed={map.roles.includes(r)} onclick={() => map.toggleRole(r)} data-testid={`codemap-role-${r}`}>{r}</button>
        {/each}
      </div>
    {/if}
  </div>

  {#if map.error}
    <p class="m-0 text-sm text-danger" data-testid="codemap-error">Could not read the code map: {map.error}</p>
  {/if}

  {#if map.level === 'modules'}
    {#if l0}
      <p class="m-0 text-xs text-ink-mute" data-testid="codemap-budget">
        {`drawing ${l0.budget.drawn} blocks for ${l0.budget.modules} modules · ${l0.budget.bands} bands for ${l0.budget.edges} edges`}{l0.coverage.unplaced ? ` · ${l0.coverage.unplaced} calls unplaced` : ''}
      </p>
      <div class="grid gap-4" style={narrow ? '' : 'grid-template-columns: 1fr 240px;'}>
        {#if narrow}
          <ModuleList data={l0} onopen={open} />
        {:else if map.lens === 'zones'}
          {#if map.zones}<ZonesLens data={map.zones} />{:else}<p class="text-sm text-ink-soft">Placing components…</p>{/if}
        {:else if map.lens === 'wheel'}
          <WheelLens data={l0} selected={map.selected} onselect={(id) => map.select(id)} onopen={open} />
        {:else if map.lens === 'matrix'}
          <MatrixLens data={l0} onopen={open} />
        {:else}
          <div class="flex flex-col gap-2">
            {#if map.lens === 'coupling' && map.cochange}
              <p class="m-0 text-xs text-ink-mute" data-testid="coupling-summary">
                {map.cochange.commits
                  ? `${map.cochange.hidden.length} pairs change together with no dependency between them · from ${map.cochange.commits} commits, ${map.cochange.minTogether}+ changes together`
                  : 'No commit history read for this workspace yet.'}
              </p>
            {/if}
            {#if map.lens === 'complexity'}
              {@const measured = l0.blocks.filter((b) => b.complexity)}
              <p class="m-0 text-xs text-ink-mute" data-testid="complexity-summary">
                {measured.length
                  ? `${measured.reduce((n, b) => n + b.complexity!.complex, 0)} of ${measured.reduce((n, b) => n + b.complexity!.functions, 0)} functions over cyclomatic 10 · ${measured.length} of ${l0.blocks.length} blocks measured (local qlty)`
                  : 'Not measured yet: complexity is measured by local qlty after a scan.'}
              </p>
            {/if}
            <ModuleMap data={l0} {bands} selected={map.selected} cycle={cycles} dashed={map.lens === 'coupling'}
              fill={map.lens === 'complexity' ? complexityFill : null} onselect={(id) => map.select(id)} onopen={open} />
          </div>
        {/if}
        {#if !narrow}
          <aside class="text-xs flex flex-col gap-2" data-testid="codemap-inspector">
            {#if picked}
              <h4 class="m-0 font-mono text-sm text-ink break-all">{picked.label}</h4>
              <p class="m-0 text-ink-mute">{picked.group} · {picked.files} files · {picked.symbols} symbols</p>
              <p class="m-0 text-ink-mute">weight out {picked.weight_out} · in {picked.weight_in}{picked.hub ? ' · hub' : ''}</p>
              {#if picked.folded.length}
                <ul class="m-0 p-0 list-none max-h-80 overflow-auto">
                  {#each picked.folded as id (id)}
                    <li><button class="bg-transparent border-none cursor-pointer p-1 font-mono text-xs text-ink" onclick={() => open(id)}>{id}</button></li>
                  {/each}
                </ul>
              {:else}
                <button class="self-start px-2 py-1 text-xs rounded border border-paper-edge bg-transparent cursor-pointer"
                  onclick={() => open(picked.id)} data-testid="codemap-open">Open files</button>
              {/if}
            {:else}
              <p class="m-0 text-ink-soft">Select a module to inspect it; double-click to open its files.</p>
            {/if}
          </aside>
        {/if}
      </div>
    {:else if map.loading}
      <p class="text-sm text-ink-soft">Loading the code map…</p>
    {/if}
  {:else if map.level === 'files' && map.l1}
    <FileColumns data={map.l1} columns={map.columns} onopen={(id, path) => void map.openFile(port, id, path)} onmodule={open} />
  {:else if map.level === 'symbols' && map.l2}
    <SymbolView data={map.l2} />
  {/if}
</div>
