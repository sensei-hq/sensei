<script lang="ts">
  // The Code map's controls (M17), in one row: what every lens shares first —
  // find a module (browse every module by group, or type to filter), group by —
  // then the lens's own: the zone (Zones), the roles shown, and the bands. Each is
  // drawn only where it changes the view. A host with its own header puts these in
  // its controls band; on its own the map draws them above the canvas.
  import { Select, Toggle } from '@rokkit/ui';
  import type { ProxyItem } from '@rokkit/states';
  import { ROLES, type CodeMapState } from './state.svelte';
  import { MIN_WEIGHTS, ROLE_BG, moduleChoices, type EdgeMode } from './view';
  import { zonesView } from './lenses';
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
  const ZONE_TONE: Record<string, string> = {
    all: 'bg-ink-mute', pain: 'bg-danger', useless: 'bg-warning', risk: 'bg-accent', main: 'bg-success',
  };

  const canvasLens = $derived(!['zones', 'wheel', 'matrix'].includes(map.lens));
  // Zones places components by their coupling: neither the grouping nor the roles move a point.
  const showsGrouping = $derived(map.level === 'modules' && map.lens !== 'zones');
  const showsRoles = $derived((map.level === 'modules' && canvasLens) || map.level === 'files');
  const showsBands = $derived(map.level === 'modules' && canvasLens && map.lens !== 'coupling');
  const choices = $derived(map.l0 ? moduleChoices(map.l0) : []);
  const zones = $derived(map.lens === 'zones' && map.level === 'modules' && map.zones ? zonesView(map.zones) : null);
  const zoneOptions = $derived(
    zones
      ? [
          { label: 'All', value: 'all', count: zones.placed.length },
          { label: 'Pain', value: 'pain', count: zones.byZone.pain },
          { label: 'Uselessness', value: 'useless', count: zones.byZone.useless },
          { label: 'Risk', value: 'risk', count: zones.byZone.risk },
          { label: 'Main sequence', value: 'main', count: zones.byZone.main },
        ]
      : [],
  );
</script>

{#snippet zoneOption(proxy: ProxyItem)}
  <span class="inline-flex items-center gap-1.5 text-xs">
    <span class={`inline-block w-2 h-2 rounded-full ${ZONE_TONE[String(proxy.value)]}`} aria-hidden="true"></span>
    {proxy.label} · {proxy.get('count')}
  </span>
{/snippet}

<div class="flex flex-wrap items-center gap-x-6 gap-y-3" data-testid="codemap-controls">
  {#if map.level !== 'symbols' && choices.length}
    <div class="w-64" data-testid="codemap-find">
      <Select items={choices} filterable filterPlaceholder="Type a module name" placeholder="Find a module" size="sm"
        onchange={(value) => typeof value === 'string' && !value.startsWith('group:') && void map.openModule(port, value)} />
    </div>
  {/if}
  {#if showsGrouping}
    <div class="flex items-center gap-2" data-testid="codemap-group">
      <span class="text-xs uppercase tracking-wide text-ink-mute">Group by</span>
      <Toggle size="sm" label="Group by" options={GROUPINGS} value={map.groupBy}
        onchange={(v) => void map.setGroupBy(port, v as GroupBy)} />
    </div>
  {/if}
  {#if zones}
    <div class="flex items-center gap-2" data-testid="codemap-zone">
      <span class="text-xs uppercase tracking-wide text-ink-mute">Zone</span>
      <Toggle size="sm" label="Zone" options={zoneOptions} value={map.zoneFilter} itemContent={zoneOption}
        onchange={(v) => (map.zoneFilter = v as typeof map.zoneFilter)} />
    </div>
  {/if}
  {#if showsRoles}
    <div class="flex items-center gap-2" role="group" aria-label="Show">
      <span class="text-xs uppercase tracking-wide text-ink-mute">Show</span>
      {#each ROLES as r (r)}
        <button class="flex items-center gap-1.5 px-2 py-1 text-xs rounded border cursor-pointer"
          class:border-paper-edge={map.roles.includes(r)} class:border-transparent={!map.roles.includes(r)}
          class:bg-paper-mute={map.roles.includes(r)} class:bg-transparent={!map.roles.includes(r)}
          class:text-ink={map.roles.includes(r)} class:text-ink-mute={!map.roles.includes(r)}
          aria-pressed={map.roles.includes(r)} onclick={() => map.toggleRole(r)} data-testid={`codemap-role-${r}`}>
          <span class={`inline-block w-2 h-2 rounded-sm ${ROLE_BG[r]}`} aria-hidden="true"></span>{r}
        </button>
      {/each}
    </div>
  {/if}
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
</div>
