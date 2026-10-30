<script lang="ts">
  // The Code map (M17): one canvas, three levels — modules, files, symbols —
  // with a breadcrumb, search, a grouping, the roles shown, band filters and
  // lenses on the same canvas, and an inspector beside it at every level. The
  // host passes the state and where to read from, and lists the lenses itself
  // (`state.showLens`); this never fetches on its own.
  import { onMount } from 'svelte';
  import { Toggle } from '@rokkit/ui';
  import { ROLES, type CodeMapState } from './state.svelte';
  import { couplingBands, cycleBands, cycleOf } from './lenses';
  import { inspect, LIBRARIES } from './inspector';
  import { listView } from './list';
  import { MIN_WEIGHTS, legendFor, searchModules, visibleBands, type EdgeMode } from './view';
  import type { CodeMapPort, GroupBy } from './types';
  import ModuleMap from './ModuleMap.svelte';
  import MatrixLens from './MatrixLens.svelte';
  import ZonesLens from './ZonesLens.svelte';
  import WheelLens from './WheelLens.svelte';
  import ModuleList from './ModuleList.svelte';
  import FileColumns from './FileColumns.svelte';
  import SymbolView from './SymbolView.svelte';
  import Inspector from './Inspector.svelte';
  import Takeaways from './Takeaways.svelte';
  import ZonesDetails from './ZonesDetails.svelte';
  import { takeaways } from './takeaways';
  import Legend from './Legend.svelte';

  let { state: map, port }: { state: CodeMapState; port: CodeMapPort } = $props();

  onMount(() => {
    if (!map.l0 && !map.loading) void map.load(port);
  });

  const GROUPINGS = [
    { label: 'Layers', value: 'layers' },
    { label: 'Folders', value: 'folders' },
  ];
  const EDGES = [
    { label: 'All bands', value: 'all' },
    { label: 'Violations & cycles', value: 'violations' },
  ];
  const WEIGHTS = MIN_WEIGHTS.map((w) => ({ label: `≥ ${w}`, value: w }));
  const ZOOMS = [
    { label: 'Fit', value: 'fit' },
    { label: '100%', value: 'actual' },
  ];

  let width = $state(1024);
  let query = $state('');
  // An unmeasured width (0) is not a narrow screen.
  const narrow = $derived(width > 0 && width < 640);
  const l0 = $derived(map.l0);
  const cycles = $derived(l0 ? cycleOf(l0) : null);
  const lensBands = $derived(
    !l0 ? [] : map.lens === 'cycles' ? cycleBands(l0) : map.lens === 'coupling' ? couplingBands(l0, map.cochange?.hidden ?? []) : l0.bands,
  );
  const bands = $derived(
    map.lens === 'coupling' ? lensBands : visibleBands(lensBands, { edgeMode: map.edgeMode, minWeight: map.minWeight }, cycles),
  );
  const canvasLens = $derived(!['zones', 'wheel', 'matrix'].includes(map.lens));
  // A control is drawn only where it changes the view: Zones places components by
  // their coupling, so neither the grouping nor the roles move a point.
  const showsGrouping = $derived(map.level === 'modules' && map.lens !== 'zones');
  const showsRoles = $derived((map.level === 'modules' && canvasLens) || map.level === 'files');
  const toAct = $derived(l0 ? takeaways(map.lens, l0, map.cochange, map.zones) : []);
  const matches = $derived(l0 ? searchModules(l0, query) : []);
  const inspection = $derived(inspect({ level: map.level, l0, l1: map.l1, l2: map.l2, selected: map.selected }));
  const open = (id: string) => {
    query = '';
    void map.openModule(port, id);
  };
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
    <div class="relative">
      <input class="w-56 px-2 py-1 text-xs rounded border border-paper-edge bg-paper text-ink"
        type="search" placeholder="Search modules" aria-label="Search modules"
        bind:value={query} onkeydown={(e) => e.key === 'Escape' && (query = '')} data-testid="codemap-search" />
      {#if matches.length}
        <ul class="absolute z-10 left-0 right-0 mt-1 m-0 p-1 list-none bg-paper border border-paper-edge rounded shadow">
          {#each matches as id (id)}
            <li><button class="w-full text-left px-2 py-1 font-mono text-xs text-ink bg-transparent border-none cursor-pointer"
              onclick={() => open(id)} data-testid={`codemap-match-${id}`}>{id}</button></li>
          {/each}
        </ul>
      {/if}
    </div>
    {#if showsGrouping}
      <div data-testid="codemap-group">
        <Toggle size="sm" label="Group by" options={GROUPINGS} value={map.groupBy}
          onchange={(v) => void map.setGroupBy(port, v as GroupBy)} />
      </div>
    {/if}
    {#if showsRoles}
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

  {#if map.level === 'modules' && l0 && canvasLens && !narrow}
    <div class="flex flex-wrap items-center gap-3">
      {#if map.lens !== 'coupling'}
        <div data-testid="codemap-edges">
          <Toggle size="sm" label="Bands" options={EDGES} value={map.edgeMode} onchange={(v) => (map.edgeMode = v as EdgeMode)} />
        </div>
        <div data-testid="codemap-weight">
          <Toggle size="sm" label="Minimum weight" options={WEIGHTS} value={map.minWeight} onchange={(v) => (map.minWeight = v as number)} />
        </div>
      {/if}
      {#if map.lens === 'structure'}
        <button class="px-2 py-1 text-xs rounded border border-paper-edge cursor-pointer text-ink"
          class:bg-paper-mute={map.byDirection} class:bg-transparent={!map.byDirection}
          aria-pressed={map.byDirection} onclick={() => (map.byDirection = !map.byDirection)} data-testid="codemap-direction">Colour by direction</button>
      {/if}
      <div data-testid="codemap-zoom">
        <Toggle size="sm" label="Zoom" options={ZOOMS} value={map.zoom} onchange={(v) => (map.zoom = v as 'fit' | 'actual')} />
      </div>
      <div class="flex-1"></div>
      <Legend entries={legendFor(map.lens, map.byDirection)} />
    </div>
  {/if}

  {#if map.level === 'modules' && l0}
    <p class="m-0 text-xs text-ink-mute" data-testid="codemap-budget">
      {`drawing ${l0.budget.drawn} blocks for ${l0.budget.modules} modules · ${bands.length} bands for ${l0.budget.edges} edges`}{l0.coverage.unplaced ? ` · ${l0.coverage.unplaced} calls unplaced` : ''}
    </p>
  {/if}

  <div class="grid gap-6" style={narrow ? '' : 'grid-template-columns: minmax(0, 1fr) 360px;'}>
    <div class="min-w-0 flex flex-col gap-2">
      {#if map.level === 'modules'}
        {#if l0}
          {#if narrow}
            <ModuleList view={listView(map.lens, l0, map.cochange, map.zones)} onopen={open} />
          {:else if map.lens === 'zones'}
            <ZonesLens data={map.zones} filter={map.zoneFilter} onopen={open} />
          {:else if map.lens === 'wheel'}
            <WheelLens data={l0} selected={map.selected} onselect={(id) => map.select(id)} onopen={open} />
          {:else if map.lens === 'matrix'}
            <MatrixLens data={l0} onopen={open} />
          {:else}
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
            <ModuleMap data={l0} {bands} selected={map.selected} cycle={map.lens === 'cycles' ? cycles : null}
              dashed={map.lens === 'coupling'} complexity={map.lens === 'complexity'} byDirection={map.byDirection}
              roles={map.roles} zoom={map.zoom} onselect={(id) => map.select(id)} onopen={open} />
          {/if}
          {#if l0.libraries.count}
            <button class="self-start px-3 py-2 text-xs rounded border border-dashed border-paper-edge bg-transparent text-ink-mute cursor-pointer"
              class:border-accent={map.selected === LIBRARIES}
              onclick={() => map.select(map.selected === LIBRARIES ? null : LIBRARIES)} data-testid="codemap-libraries">
              {l0.libraries.count} referenced libraries · not members
            </button>
          {/if}
        {:else if map.loading}
          <p class="text-sm text-ink-soft">Loading the code map…</p>
        {/if}
      {:else if map.level === 'files' && map.l1}
        <FileColumns data={map.l1} columns={map.columns} onopen={(id, path) => void map.openFile(port, id, path)} onmodule={open} />
      {:else if map.level === 'symbols' && map.l2}
        <SymbolView data={map.l2} />
      {/if}
    </div>
    {#if !narrow}
      <div class="flex flex-col gap-6 min-w-0" data-testid="codemap-side">
        {#if map.level === 'modules' && map.lens === 'zones'}
          {#if map.zones}
            <ZonesDetails data={map.zones} filter={map.zoneFilter} onfilter={(f) => (map.zoneFilter = f)} onopen={open} />
          {/if}
          <Takeaways items={toAct} onopen={open} />
        {:else if map.level === 'modules' && !map.selected}
          <Takeaways items={toAct} onopen={open} />
        {:else}
          <Inspector data={inspection} onopen={open} />
        {/if}
      </div>
    {/if}
  </div>
</div>
