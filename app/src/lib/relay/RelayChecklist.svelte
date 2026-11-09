<script lang="ts">
  // Relay · the steps checklist (F8): the holder's turn narrated step by
  // step. Completed checked, the first open step highlighted as current,
  // skipped struck with its reason. Empty reads honestly — the lane posted
  // nothing (D83), never a fabricated zero.
  import type { RelayTaskStep } from '$lib/relay-api.js';
  import { currentStepKey } from './relay-format.js';

  let { steps }: { steps: RelayTaskStep[] } = $props();

  const current = $derived(currentStepKey(steps));
</script>

<section class="flex flex-col gap-1" data-checklist>
  <h3 class="text-xs uppercase tracking-wide text-ink-mute m-0">Steps</h3>
  {#if steps.length === 0}
    <p class="text-xs text-ink-faint m-0">No steps posted yet</p>
  {:else}
    {#each steps as step (step.key)}
      <div
        class="flex items-baseline gap-2 rounded-sm px-2 py-1 {step.key === current ? 'bg-paper-soft' : ''}"
        data-step={step.key}
        data-step-state={step.state}
        data-step-current={step.key === current ? 'true' : undefined}
      >
        {#if step.state === 'done'}
          <span class="text-xs text-success shrink-0" data-step-check>✓</span>
          <span class="text-sm text-ink-mute m-0">{step.title}</span>
        {:else if step.state === 'skipped'}
          <span class="text-xs text-ink-faint shrink-0">–</span>
          <span class="text-sm text-ink-faint m-0"><s>{step.title}</s></span>
          {#if step.reason}<span class="text-xs text-ink-faint m-0">{step.reason}</span>{/if}
        {:else}
          <span class="text-xs text-ink-mute shrink-0">○</span>
          <span class="text-sm m-0 {step.key === current ? 'font-medium' : 'text-ink-soft'}">{step.title}</span>
        {/if}
      </div>
    {/each}
  {/if}
</section>
