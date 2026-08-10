<script lang="ts">
  // Global status strip shown only while the daemon reports its DB connection as
  // degraded (it lost the cold-boot race and is self-healing). Pure: the mode
  // comes from daemon-health.svelte.ts, which polls the daemon's /health.
  import type { DaemonDbMode } from '$lib/health-types.js';

  let { mode }: { mode?: DaemonDbMode } = $props();
</script>

{#if mode === 'degraded'}
  <div
    data-component="daemon-status-banner"
    role="status"
    class="flex items-center gap-2 border-b border-warning-soft bg-warning-soft px-4 py-1.5 text-xs text-ink"
  >
    <span>
      Reconnecting to the database — some data may be briefly unavailable while the
      daemon recovers.
    </span>
  </div>
{/if}
