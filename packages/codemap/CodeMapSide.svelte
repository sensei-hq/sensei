<script lang="ts">
  // The Code map's side panel, content only: Zones' details, the inspector, and
  // what to act on. `CodeMap` frames it beside the canvas; a host that lays the
  // panel out itself (the diagrams put it in the ScreenShell's sidebar) renders
  // this where its own frame goes. One reader of the state either way, so the
  // panel cannot disagree with the canvas beside it.
  import type { CodeMapState } from './state.svelte';
  import type { CodeMapPort } from './types';
  import { inspect } from './inspector';
  import { takeaways } from './takeaways';
  import Inspector from './Inspector.svelte';
  import Takeaways from './Takeaways.svelte';
  import ZonesDetails from './ZonesDetails.svelte';

  let {
    state: map,
    port,
    label = 'Code map',
  }: {
    state: CodeMapState;
    /** Where to read a module's files, when the panel opens one. */
    port: CodeMapPort;
    /** The diagram's name, heading what to act on when it is copied. */
    label?: string;
  } = $props();

  // The daemon narrowed the zoomed lenses to the focus; the panel reads the same.
  const zones = $derived(map.focusedZones);
  const complexity = $derived(map.focusedComplexity);
  const cochange = $derived(map.focusedCochange);
  const toAct = $derived(map.l0 ? takeaways(map.lens, map.l0, cochange, zones, complexity) : []);
  const focused = $derived(map.focus?.kind === 'module' ? map.focus.id : null);
  const inspection = $derived(inspect({ level: map.level, l0: map.l0, l1: map.l1, l2: map.l2, selected: map.selected ?? focused }));
  /** A click on a module in any lens focuses it (user, 2026-11-03). */
  const focusOn = (id: string) => void map.setFocus(port, { kind: 'module', id });
  /** Open a module's files — the panel's explicit action, not a click on a lens. */
  const open = (id: string) => void map.openModule(port, id);
</script>

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
