<script lang="ts">
  // One lane on Now's By agent view (#422): its codename (its code beside it when it has
  // one), its assistant and model, its state; then the run tasks it ran, in the order it
  // ran them, and what it holds now.
  import { Pill } from '@rokkit/ui';
  import type { RelayAgentGroupView } from '@sensei/state';
  import RunTaskList from './RunTaskList.svelte';

  let { agent }: { agent: RelayAgentGroupView } = $props();
</script>

<section data-agent-group={agent.id} aria-label={agent.name} class="flex flex-col gap-1.5">
  <div class="flex flex-wrap items-center gap-x-2 gap-y-1">
    <span data-agent-name class="text-sm font-medium text-ink">{agent.name}</span>
    {#if agent.name !== agent.code}<span class="font-mono text-xs text-ink-mute">{agent.code}</span>{/if}
    <span data-agent-who class="text-xs text-ink-soft">{agent.who}</span>
    <span class="ml-auto"><Pill value={agent.state.label} tone={agent.state.tone} size="sm" /></span>
  </div>
  {#if agent.tasks.length > 0}
    <RunTaskList rows={agent.tasks} label="Run tasks · {agent.name}" />
  {:else}
    <p data-agent-empty class="m-0 text-xs text-ink-mute">No run tasks yet</p>
  {/if}
</section>
