<script lang="ts">
  // One open decision, as the mockup's card (COMPONENTS §3b DecisionCard; CONTRACT
  // §1c-1, §3e): the question, what it blocks with a link to that task, its options
  // with this decision's own recommendation marked, and Answer. One write per press;
  // a refusal says why and leaves the card where it was.
  import { untrack } from 'svelte';
  import { Notice } from '@sensei/ui';
  import type { RelayDecision } from '$lib/relay-api.js';

  let {
    decision,
    href,
    onanswer,
  }: {
    decision: RelayDecision;
    /** The task the question blocks, when it names one. */
    href: string | null;
    onanswer: (option: string) => Promise<string | null>;
  } = $props();

  // The card's own choice, seeded once from the decision it was drawn for.
  let chosen = $state(untrack(() => decision.recommended ?? decision.options[0]?.id ?? ''));
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function send(): Promise<void> {
    if (busy || !chosen) return;
    busy = true;
    error = null;
    const refused = await onanswer(chosen);
    if (refused) error = refused;
    busy = false;
  }
</script>

<article
  data-decision
  class="flex flex-col gap-2 p-3.5 rounded-xl border border-solid border-warning bg-[color-mix(in_oklch,var(--warning)_8%,var(--paper))]"
>
  <strong data-decision-question class="text-base font-semibold">{decision.question}</strong>
  <span class="text-sm text-ink-soft">
    {#if decision.context}{decision.context}{/if}
    {#if href}<a class="text-accent no-underline hover:underline" {href}>Open the task ›</a>{/if}
  </span>
  <div class="flex flex-wrap items-center gap-1.5">
    {#each decision.options as option (option.id)}
      {@const recommended = decision.recommended === option.id}
      <button
        type="button"
        data-option={option.id}
        data-recommended={recommended ? '' : undefined}
        aria-pressed={chosen === option.id}
        onclick={() => (chosen = option.id)}
        class={[
          'inline-flex items-center gap-1.5 min-h-[34px] px-3 rounded-lg border border-solid bg-paper text-sm text-ink cursor-pointer',
          recommended ? 'border-accent' : 'border-paper-edge',
        ]}
      >
        <span
          aria-hidden="true"
          class={[
            'inline-flex items-center justify-center w-3.5 h-3.5 rounded-full border border-solid',
            chosen === option.id ? 'border-accent' : 'border-paper-edge-strong',
          ]}
        >{#if chosen === option.id}<span class="w-2 h-2 rounded-full bg-accent"></span>{/if}</span>
        <span>{option.label}</span>
        {#if recommended}<span class="text-xs text-accent">recommended</span>{/if}
      </button>
    {/each}
    <button
      type="button"
      data-decision-answer
      disabled={busy || !chosen}
      onclick={() => void send()}
      class="min-h-[34px] px-3.5 rounded-lg border-0 bg-ink text-paper text-sm cursor-pointer disabled:opacity-60"
    >Answer</button>
  </div>
  {#if error}<Notice size="inline" tone="fault" title={error} data-decision-error />{/if}
</article>
