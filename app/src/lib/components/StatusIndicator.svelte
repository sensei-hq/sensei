<script lang="ts">
  import type { ComponentStatus } from '$lib/health-types.js';
  import { StatusDisc } from '$lib/components';
  import { componentStatusView } from '$lib/health-status';

  interface Props {
    status: ComponentStatus;
    label?: string;
  }
  let { status, label }: Props = $props();

  // Label and colour are componentStatusView's — the same view the disc and the
  // gate numeral use, so one row cannot disagree with itself.
  const view = $derived(componentStatusView(status, label));
</script>

<div class="inline-flex items-center gap-2 shrink-0">
  {#if view.label}
    <span
      data-component="status-indicator-label"
      class="font-mono text-xs uppercase tracking-wide leading-none {view.text}"
    >{view.label}</span>
  {/if}
  <StatusDisc {status} size={20} />
</div>
