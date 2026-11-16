<script lang="ts">
  // One open run on Now (#422): its band, then its tasks waiting (by priority), in
  // progress, and done (in the order they ran). An empty part is not drawn.
  import type { RelayRunGroupView } from '@sensei/state';
  import SprintBand from './SprintBand.svelte';
  import RunTaskList from './RunTaskList.svelte';

  let { group }: { group: RelayRunGroupView } = $props();
  const STAGES = [
    { key: 'waiting', label: 'Waiting' },
    { key: 'active', label: 'In progress' },
    { key: 'done', label: 'Done' },
  ] as const;
</script>

<section data-run-group={group.id} aria-label={group.run.title} class="flex flex-col gap-2">
  <SprintBand run={group.run} />
  {#each STAGES as stage (stage.key)}
    {#if group[stage.key].length > 0}
      <div data-run-stage={stage.key} class="flex flex-col gap-1">
        <p class="m-0 text-xs text-ink-mute">{stage.label} · {group[stage.key].length}</p>
        <RunTaskList rows={group[stage.key]} label="{stage.label} · {group.run.title}" />
      </div>
    {/if}
  {/each}
</section>
