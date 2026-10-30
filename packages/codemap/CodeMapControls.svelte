<script lang="ts">
  // The Code map's controls (M17), each drawn only where it changes the view:
  // jump to a module; group by; the roles shown; and the bands — colour by
  // direction (Structure), all or violations & cycles, and a minimum weight.
  // A host with its own header puts these in its controls band; on its own the
  // map draws them above the canvas.
  import { Toggle } from '@rokkit/ui';
  import { ROLES, type CodeMapState } from './state.svelte';
  import { MIN_WEIGHTS, ROLE_BG, searchModules, type EdgeMode } from './view';
  import type { CodeMapPort, GroupBy } from './types';

  let { state: map, port }: { state: CodeMapState; port: CodeMapPort } = $props();

  const GROUPINGS = [
    { label: 'Layers', value: 'layers' },
    { label: 'Folders', value: 'folders' },
  ];
  const EDGES = [
    { label: 'All bands', value: 'all' },
    { label: 'Violations & cycles', value: 'violations' },
  ];
  const WEIGHTS = MIN_WEIGHTS.map((w) => ({ label: `≥ ${w}`, value: w }));

  let query = $state('');
  const canvasLens = $derived(!['zones', 'wheel', 'matrix'].includes(map.lens));
  // Zones places components by their coupling: neither the grouping nor the roles move a point.
  const showsGrouping = $derived(map.level === 'modules' && map.lens !== 'zones');
  const showsRoles = $derived((map.level === 'modules' && canvasLens) || map.level === 'files');
  const showsBands = $derived(map.level === 'modules' && canvasLens && map.lens !== 'coupling');
  const matches = $derived(map.l0 ? searchModules(map.l0, query) : []);
  const open = (id: string) => {
    query = '';
    void map.openModule(port, id);
  };
</script>

<div class="flex flex-col gap-3" data-testid="codemap-controls">
  <div class="flex flex-wrap items-center gap-x-6 gap-y-2">
    {#if showsGrouping}
      <div class="flex items-center gap-2" data-testid="codemap-group">
        <span class="text-xs uppercase tracking-wide text-ink-mute">Group by</span>
        <Toggle size="sm" label="Group by" options={GROUPINGS} value={map.groupBy}
          onchange={(v) => void map.setGroupBy(port, v as GroupBy)} />
      </div>
    {/if}
    {#if showsRoles}
      <div class="flex items-center gap-2" role="group" aria-label="Show">
        <span class="text-xs uppercase tracking-wide text-ink-mute">Show</span>
        {#each ROLES as r (r)}
          <button class="flex items-center gap-1.5 px-2 py-1 text-xs rounded border cursor-pointer text-ink"
            class:border-paper-edge={map.roles.includes(r)} class:border-transparent={!map.roles.includes(r)}
            class:bg-paper-mute={map.roles.includes(r)} class:bg-transparent={!map.roles.includes(r)}
            class:text-ink-mute={!map.roles.includes(r)}
            aria-pressed={map.roles.includes(r)} onclick={() => map.toggleRole(r)} data-testid={`codemap-role-${r}`}>
            <span class={`inline-block w-2 h-2 rounded-sm ${ROLE_BG[r]}`} aria-hidden="true"></span>{r}
          </button>
        {/each}
      </div>
    {/if}
  </div>
  <div class="flex flex-wrap items-center gap-x-6 gap-y-2">
    {#if showsBands}
      <div class="flex flex-wrap items-center gap-2">
        <span class="text-xs uppercase tracking-wide text-ink-mute">Bands</span>
        {#if map.lens === 'structure'}
          <button class="px-2 py-1 text-xs rounded border border-paper-edge cursor-pointer text-ink"
            class:bg-paper-mute={map.byDirection} class:bg-transparent={!map.byDirection}
            aria-pressed={map.byDirection} onclick={() => (map.byDirection = !map.byDirection)} data-testid="codemap-direction">Colour by direction</button>
        {/if}
        <div data-testid="codemap-edges">
          <Toggle size="sm" label="Bands" options={EDGES} value={map.edgeMode} onchange={(v) => (map.edgeMode = v as EdgeMode)} />
        </div>
        <div data-testid="codemap-weight">
          <Toggle size="sm" label="Minimum weight" options={WEIGHTS} value={map.minWeight} onchange={(v) => (map.minWeight = v as number)} />
        </div>
      </div>
    {/if}
    <div class="flex-1"></div>
    {#if map.level !== 'symbols'}
      <div class="relative">
        <input class="w-64 px-3 py-1.5 text-sm rounded border border-paper-edge bg-paper text-ink"
          type="search" placeholder="Jump to a module" aria-label="Jump to a module"
          bind:value={query} onkeydown={(e) => e.key === 'Escape' && (query = '')} data-testid="codemap-search" />
        {#if matches.length}
          <ul class="absolute z-20 left-0 right-0 mt-1 m-0 p-1 list-none bg-paper border border-paper-edge rounded shadow">
            {#each matches as id (id)}
              <li><button class="w-full text-left px-2 py-1 font-mono text-xs text-ink bg-transparent border-none cursor-pointer"
                onclick={() => open(id)} data-testid={`codemap-match-${id}`}>{id}</button></li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}
  </div>
</div>
