<script lang="ts">
  // One open decision (CONTRACT §1c, §3e): the question, what it is blocking, its
  // options with this decision's own recommendation marked, and Answer. The answer
  // is one write per press; a refusal says why and leaves the row where it was.
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

  // The row's own choice, seeded once from the decision it was drawn for.
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

<article class="rounded-sm border border-paper-edge bg-paper-soft p-3 flex flex-col gap-2" data-decision>
  <p class="text-sm font-medium m-0">{decision.question}</p>
  {#if decision.context}<p class="text-xs text-ink-mute m-0">{decision.context}</p>{/if}

  <div class="flex flex-wrap gap-2">
    {#each decision.options as option (option.id)}
      <button
        type="button"
        class="rounded-sm border px-2 py-1 text-xs"
        class:border-accent={chosen === option.id}
        class:text-ink={chosen === option.id}
        class:border-paper-edge={chosen !== option.id}
        class:text-ink-mute={chosen !== option.id}
        data-option={option.id}
        data-recommended={decision.recommended === option.id ? '' : undefined}
        aria-pressed={chosen === option.id}
        onclick={() => (chosen = option.id)}
      >
        {option.label}{#if decision.recommended === option.id} · recommended{/if}
      </button>
    {/each}
  </div>

  <div class="flex items-center gap-3">
    {#if href}<a class="text-xs underline" href={href}>{decision.task_title ?? 'Its task'}</a>{/if}
    <button
      type="button"
      class="ml-auto rounded-sm bg-accent px-2 py-1 text-xs text-on-accent disabled:opacity-60"
      data-decision-answer
      disabled={busy || !chosen}
      onclick={() => void send()}
    >
      Answer
    </button>
  </div>
  {#if error}<Notice size="inline" tone="fault" title={error} data-decision-error />{/if}
</article>
