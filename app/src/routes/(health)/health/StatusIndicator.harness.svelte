<script lang="ts">
  import StatusIndicator from './StatusIndicator.svelte';
  import type { ComponentStatus } from '$lib/health-types.js';

  let { status, label }: {
    status: ComponentStatus;
    label?: string;
  } = $props();
</script>

<StatusIndicator {status} {label} />
