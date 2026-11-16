<script lang="ts">
  // Lanes · N (COMPONENTS §3b LaneList; CONTRACT §1c-2; #422). By run: each crew's open
  // runs (RunGroup), then its lanes on no run task as lane rows; by agent: one group per
  // lane. The main window draws each crew under its repository, because a lane code is
  // unique per crew only. With no open run the lanes are drawn alone, as before.
  // Lane rows are a display-only rokkit List: a lane on a task is a link row, an idle
  // lane a plain row (no button, never focused).
  import { List, Toggle } from '@rokkit/ui';
  import type { ProxyItem } from '@rokkit/states';
  import type { RelayCrewAgentsView, RelayCrewRunsView, RelayLaneRowView, RelayLaneView } from '@sensei/state';
  import LaneRow from './LaneRow.svelte';
  import SectionHeading from './SectionHeading.svelte';
  import RunGroup from './RunGroup.svelte';
  import AgentGroup from './AgentGroup.svelte';

  let {
    crews,
    agents = [],
    view = 'run',
    onview = null,
    showing = null,
  }: {
    crews: RelayCrewRunsView[];
    agents?: RelayCrewAgentsView[];
    view?: RelayLaneView;
    /** The person picks By run or By agent; no callback, no switch. */
    onview?: ((view: RelayLaneView) => void) | null;
    showing?: { label: string; onclear: () => void } | null;
  } = $props();

  // Every lane once: the agent groups hold each lane; without them, the rows drawn.
  const count = $derived(
    agents.length > 0 ? agents.reduce((n, c) => n + c.agents.length, 0) : crews.reduce((n, c) => n + c.idle.length, 0),
  );
  const VIEWS = [
    { value: 'run', label: 'By run' },
    { value: 'agent', label: 'By agent' },
  ];
</script>

{#snippet laneRows(rows: RelayLaneRowView[], label: string)}
  <List items={rows} fields={{ value: 'id' }} selectable={false} variant="card" {label}>
    {#snippet itemContent(proxy: ProxyItem)}<LaneRow row={proxy.original as RelayLaneRowView} />{/snippet}
  </List>
{/snippet}

<section aria-label="Lanes" data-relay-section="lanes" class="flex flex-col gap-1.5">
  <div class="flex flex-wrap items-center gap-2">
    <SectionHeading label="Lanes" {count} {showing} />
    {#if onview}
      <div data-lane-view class="ml-auto">
        <Toggle size="sm" options={VIEWS} value={view} label="Group the lanes" onchange={(v: unknown) => onview(v as RelayLaneView)} />
      </div>
    {/if}
  </div>
  {#if view === 'agent'}
    {#each agents as crew (crew.id)}
      <div data-crew-group={crew.id} class="flex flex-col gap-3">
        {#if crew.label}<p data-crew-heading class="m-0 text-xs text-ink-mute">{crew.label}</p>{/if}
        {#each crew.agents as agent (agent.id)}<AgentGroup {agent} />{/each}
      </div>
    {/each}
  {:else}
    {#each crews as crew (crew.id)}
      <div data-crew-group={crew.id} class="flex flex-col gap-3">
        {#if crew.label}<p data-crew-heading class="m-0 text-xs text-ink-mute">{crew.label}</p>{/if}
        {#each crew.runs as group (group.id)}<RunGroup {group} />{/each}
        {#if crew.idle.length > 0}
          <div data-lanes-idle class="flex flex-col gap-1">
            {#if crew.runs.length > 0}<p data-lanes-idle-label class="m-0 text-xs text-ink-mute">Not on a run task</p>{/if}
            {@render laneRows(crew.idle, crew.label ?? 'Lanes')}
          </div>
        {/if}
      </div>
    {/each}
  {/if}
</section>
