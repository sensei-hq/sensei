<script lang="ts">
  import type { Component } from '$lib/health-types.js';
  import StatusIndicator from './StatusIndicator.svelte';
  import { componentStatusView } from '$lib/health-status';

  interface Props {
    gate: Component;
    numeral: string;
  }
  let { gate, numeral }: Props = $props();

  // The numeral's colour, the label and the dimming are componentStatusView's.
  const view = $derived(componentStatusView(gate.status, gate.installingVerb));
</script>

<div
  data-component="gate-row"
  data-gate-id={gate.id}
  data-gate-status={gate.status}
  class="gate-row border-b border-paper-edge py-2.5"
  style="opacity: {view.dim ? 0.65 : 1};"
>
  <div class="grid grid-cols-[22px_1fr_auto] items-center gap-3">
    <span
      data-component="gate-row-numeral"
      class="font-kanji text-lg text-center leading-none {view.text}"
    >{numeral}</span>

    <div class="min-w-0 flex flex-col gap-1">
      <div class="flex items-baseline gap-2 flex-wrap leading-tight">
        <span class="font-display text-sm font-medium text-ink">{gate.label}</span>
        {#if gate.detail}
          <span class="text-xs text-ink-faint">· {gate.detail}</span>
        {/if}
        {#if gate.version}
          <span class="font-mono text-xs text-ink-soft">{gate.version}</span>
        {/if}
      </div>
      <div
        data-component="gate-row-description"
        class="italic text-xs text-ink-mute leading-snug"
      >{gate.description}</div>
    </div>

    <StatusIndicator status={gate.status} label={gate.installingVerb} />
  </div>
</div>
