<script lang="ts">
  import type { ComponentStatus } from '$lib/health-types.js';
  import { StatusDisc } from '$lib/components';

  interface Props {
    status: ComponentStatus;
    label?: string;
  }
  let { status, label }: Props = $props();

  const defaultLabel = $derived(
    status === 'ready'    ? 'ready'
    : status === 'failed' ? 'blocked'
    : status === 'checking'   ? 'checking'
    : status === 'installing' ? 'installing'
    :                            null,  // pending — no label
  );

  const displayLabel = $derived(label ?? defaultLabel);

  const labelTone = $derived(
    status === 'ready'                              ? 'text-success'
    : status === 'checking' || status === 'installing' ? 'text-accent'
    : status === 'failed'                            ? 'text-accent'
    :                                                  'text-ink-faint',
  );
</script>

<div class="inline-flex items-center gap-2 shrink-0">
  {#if displayLabel}
    <span
      data-component="status-indicator-label"
      class="font-mono text-xs uppercase tracking-wide leading-none {labelTone}"
    >{displayLabel}</span>
  {/if}
  <StatusDisc {status} size={20} />
</div>
