<script lang="ts">
  import type { ComponentStatus } from '$lib/health-types.js';
  import Spinner from './Spinner.svelte';

  interface Props {
    status: ComponentStatus;
    size?: number;
  }
  let { status, size = 20 }: Props = $props();

  const borderClass = $derived(
    status === 'ready'                                ? 'border-success'
    : status === 'checking' || status === 'installing'? 'border-accent'
    : status === 'failed'                              ? 'border-accent'
    :                                                    'border-ink-faint',
  );

  const innerSize = $derived(size >= 32 ? 14 : size >= 24 ? 11 : 10);
  const strokeWidth = $derived(size >= 32 ? 2 : 1.5);
</script>

<span
  data-component="status-disc"
  data-status={status}
  class="inline-flex items-center justify-center rounded-full bg-paper border-[1.5px] shrink-0 {borderClass}"
  style="width: {size}px; height: {size}px;"
>
  {#if status === 'ready'}
    <svg width={innerSize} height={innerSize} viewBox="0 0 10 10" fill="none" aria-hidden="true">
      <path d="M2 5.2 L4.2 7.2 L8 3" stroke="currentColor"
            stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round"
            class="text-success" />
    </svg>
  {:else if status === 'checking' || status === 'installing'}
    <Spinner size={innerSize} tone="accent" />
  {:else if status === 'failed'}
    <span class="font-kanji text-accent leading-none" style="font-size: {Math.round(size * 0.6)}px;">?</span>
  {/if}
</span>
