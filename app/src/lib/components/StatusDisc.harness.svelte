<script lang="ts">
  import StatusDisc from './StatusDisc.svelte';
  import type { ComponentStatus } from '$lib/health-types.js';

  let { status, size = 20 }: {
    status: ComponentStatus;
    size?: number;
  } = $props();
</script>

<StatusDisc {status} {size} />
