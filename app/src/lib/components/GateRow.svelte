<script lang="ts">
  import type { Component } from '$lib/health-types.js';
  import StatusIndicator from './StatusIndicator.svelte';

  interface Props {
    gate: Component;
    numeral: string;
  }
  let { gate, numeral }: Props = $props();

  const numeralTone = $derived(
    gate.status === 'ready'                                ? 'text-success'
    : gate.status === 'failed'                              ? 'text-accent'
    : gate.status === 'checking' || gate.status === 'installing' ? 'text-ink-mute'
    :                                                          'text-ink-faint',
  );

  const indicatorLabel = $derived(
    gate.status === 'installing' ? gate.installingVerb : undefined,
  );
</script>

<div
  data-component="gate-row"
  data-gate-id={gate.id}
  class="gate-row border-b border-paper-edge py-2.5"
  style="opacity: {gate.status === 'pending' ? 0.65 : 1};"
>
  <div class="grid grid-cols-[22px_1fr_auto] items-center gap-3">
    <span
      data-component="gate-row-numeral"
      class="font-kanji text-lg text-center leading-none {numeralTone}"
    >{numeral}</span>

    <div class="min-w-0 flex flex-col gap-1">
      <div class="flex items-baseline gap-2 flex-wrap leading-tight">
        <span class="font-display text-sm font-medium text-ink">{gate.label}</span>
        {#if gate.detail}
          <span class="text-xs text-ink-faint">· {gate.detail}</span>
        {/if}
        {#if gate.version}
          <span class="font-mono text-xs text-ink-mute">{gate.version}</span>
        {/if}
      </div>
      <div
        data-component="gate-row-description"
        class="italic text-xs text-ink-soft leading-snug"
      >{gate.description}</div>
    </div>

    <StatusIndicator status={gate.status} label={indicatorLabel} />
  </div>
</div>
