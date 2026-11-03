<script lang="ts">
  // The Code map (M17): one canvas, three levels — modules, files, symbols —
  // with lenses on the same canvas, rokkit's zoom over each drawing, and a side
  // panel beside it at every level. The host passes the state and where to read
  // from, and lists the lenses itself (`state.showLens`); this never fetches on
  // its own. A host with its own header takes the controls (`CodeMapControls`)
  // and the side panel out.
  import { onMount } from 'svelte';
  import type { CodeMapState } from './state.svelte';
  import { inspect, LIBRARIES } from './inspector';
  import { listView } from './list';
  import { budgetLine, drawnBands, legendFor, segmentLabel } from './view';
  import type { CodeMapPort } from './types';
  import ModuleMap from './ModuleMap.svelte';
  import MatrixLens from './MatrixLens.svelte';
  import ZonesLens from './ZonesLens.svelte';
  import WheelLens from './WheelLens.svelte';
  import CyclesLens from './CyclesLens.svelte';
  import CouplingLens from './CouplingLens.svelte';
  import ComplexityLens from './ComplexityLens.svelte';
  import ModuleList from './ModuleList.svelte';
  import FileColumns from './FileColumns.svelte';
  import SymbolView from './SymbolView.svelte';
  import Inspector from './Inspector.svelte';
  import Takeaways from './Takeaways.svelte';
  import ZonesDetails from './ZonesDetails.svelte';
  import ZoomFrame from './ZoomFrame.svelte';
  import CodeMapControls from './CodeMapControls.svelte';
  import { takeaways } from './takeaways';
  import Legend from './Legend.svelte';

  let {
    state: map,
    port,
    controls = true,
    side = true,
    label = 'Code map',
  }: {
    state: CodeMapState;
    port: CodeMapPort;
    /** Draw the controls above the canvas (false: the host's header has them). */
    controls?: boolean;
    /** Draw the side panel — takeaways, inspector, Zones' details. */
    side?: boolean;
    /** The diagram's name, heading what to act on when it is copied. */
    label?: string;
  } = $props();

  onMount(() => {
    if (!map.l0 && !map.loading) void map.load(port);
  });

  let width = $state(1024);
  // An unmeasured width (0) is not a narrow screen.
  const narrow = $derived(width > 0 && width < 640);
  const l0 = $derived(map.l0);
  // The daemon narrowed the module map to the focus; the state narrows the rest.
  const cochange = $derived(map.focusedCochange);
  const complexity = $derived(map.focusedComplexity);
  const zones = $derived(map.focusedZones);
  /** The focus as the empty states name it. */
  const focusLabel = $derived(map.focus ? (map.focus.kind === 'module' ? map.focus.id : segmentLabel(map.groupBy, map.focus.id)) : null);
  const bands = $derived(l0 ? drawnBands(map.lens, l0, cochange, { edgeMode: map.edgeMode, minWeight: map.minWeight }) : []);
  const toAct = $derived(l0 ? takeaways(map.lens, l0, cochange, zones, complexity) : []);
  // A focused module is what the side panel inspects when nothing else is selected.
  const focused = $derived(map.focus?.kind === 'module' ? map.focus.id : null);
  const inspection = $derived(inspect({ level: map.level, l0, l1: map.l1, l2: map.l2, selected: map.selected ?? focused }));
  /** Open a module's files — the side panel's explicit action, not a click on a lens. */
  const open = (id: string) => void map.openModule(port, id);
  /** The narrow list stands in for Zones and Hidden coupling from their own data;
   *  until it is read (or when it failed) the list says so, never an empty list. */
  const listNeeds = $derived((map.lens === 'zones' && !zones) || (map.lens === 'coupling' && !cochange));
  /** Read the lens's own data again after it failed: showing it re-reads what is missing. */
  const retry = () => void map.showLens(port, map.lens);
  /** A click on a module in any lens focuses it (user, 2026-11-03). */
  const focusOn = (id: string) => void map.setFocus(port, { kind: 'module', id });
  const zoomTo = (z: number) => (map.zoom = z);
</script>

<div class="h-full min-h-0 flex flex-col gap-3" bind:clientWidth={width} data-testid="codemap">
  {#if controls}
    <CodeMapControls state={map} {port} />
  {/if}
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
    {#if map.level === 'modules'}<Legend entries={legendFor(map.lens, map.byDirection)} />{/if}
  </div>

  {#if map.error}
    <p class="m-0 text-sm text-danger" data-testid="codemap-error">Could not read the code map: {map.error}</p>
  {/if}

  {#if controls && map.level === 'modules' && l0}
    <p class="m-0 text-xs text-ink-mute" data-testid="codemap-budget">{budgetLine(l0, bands.length)}</p>
  {/if}

  <!-- The diagram and the side panel each scroll on their own; each takes focus so
       the keyboard can scroll it (axe scrollable-region-focusable). -->
  <div class="flex-1 min-h-0 grid gap-6" style={narrow || !side ? '' : 'grid-template-columns: minmax(0, 1fr) 360px;'}>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="min-w-0 min-h-0 overflow-auto flex flex-col gap-2" tabindex="0" role="region" aria-label="Diagram" data-testid="codemap-canvas">
      {#if map.level === 'modules'}
        {#if l0}
          {#if narrow}
            <ModuleList view={listView(map.lens, l0, cochange, zones)} onfocus={focusOn}
              failed={listNeeds ? map.dataError : null} pending={listNeeds && !map.dataError} onretry={retry} />
          {:else if map.lens === 'zones'}
            <ZoomFrame zoom={map.zoom} onzoom={zoomTo}>
              <ZonesLens data={zones} filter={map.zoneFilter} zoom={map.zoom} onfocus={focusOn} failed={map.dataError} onretry={retry} />
            </ZoomFrame>
          {:else if map.lens === 'wheel'}
            <ZoomFrame zoom={map.zoom} onzoom={zoomTo}>
              <WheelLens data={l0} selected={map.selected ?? focused} zoom={map.zoom} onselect={(id) => map.select(id)} onfocus={focusOn} />
            </ZoomFrame>
          {:else if map.lens === 'coupling'}
            {#if cochange?.commits}
              <p class="m-0 text-xs text-ink-mute" data-testid="coupling-summary">
                {`${cochange.hidden.length} pairs change together with no dependency between them · from ${cochange.commits} commits, ${cochange.minTogether}+ changes together`}
              </p>
            {/if}
            <ZoomFrame zoom={map.zoom} onzoom={zoomTo}>
              <CouplingLens data={cochange} zoom={map.zoom} onfocus={focusOn} focus={focusLabel} failed={map.dataError} onretry={retry} />
            </ZoomFrame>
          {:else if map.lens === 'complexity'}
            <ZoomFrame zoom={map.zoom} onzoom={zoomTo}>
              <ComplexityLens data={complexity} zoom={map.zoom} onfocus={focusOn} focus={focusLabel} failed={map.dataError} onretry={retry} />
            </ZoomFrame>
          {:else if map.lens === 'cycles'}
            <ZoomFrame zoom={map.zoom} onzoom={zoomTo}>
              <CyclesLens data={l0} zoom={map.zoom} onfocus={focusOn} />
            </ZoomFrame>
          {:else if map.lens === 'matrix'}
            <ZoomFrame zoom={map.zoom} onzoom={zoomTo}>
              <MatrixLens data={l0} zoom={map.zoom} onfocus={focusOn} />
            </ZoomFrame>
          {:else}
            <ZoomFrame zoom={map.zoom} onzoom={zoomTo}>
              <ModuleMap data={l0} {bands} selected={map.selected ?? focused}
                byDirection={map.byDirection}
                roles={map.roles} zoom={map.zoom} onselect={(id) => map.select(id)} onfocus={focusOn} />
            </ZoomFrame>
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
    {#if side && !narrow}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div class="flex flex-col gap-6 min-w-0 min-h-0 overflow-auto pl-6 border-l border-paper-edge" tabindex="0" role="region" aria-label="Side panel" data-testid="codemap-side">
        {#if map.level === 'modules' && map.lens === 'zones'}
          {#if zones}
            <ZonesDetails data={zones} filter={map.zoneFilter} onfocus={focusOn} />
          {/if}
          <Takeaways items={toAct} onfocus={focusOn} {label} />
        {:else if map.level === 'modules' && !map.selected && !map.focus}
          <Takeaways items={toAct} onfocus={focusOn} {label} />
        {:else}
          <Inspector data={inspection} onopen={open}
            onclear={map.focus && map.level === 'modules' ? () => void map.clearFocus(port) : undefined} />
          {#if map.level === 'modules' && map.focus && !map.selected}
            <Takeaways items={toAct} onfocus={focusOn} {label} />
          {/if}
        {/if}
      </div>
    {/if}
  </div>
</div>
