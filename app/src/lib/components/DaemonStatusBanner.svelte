<script lang="ts">
  // Global status strip, shown only while something is wrong with the daemon:
  // its DB connection degraded (it lost the cold-boot race and is self-healing),
  // or the settings read failed (appState keeps the last ones and retries).
  // Pure: the mode comes from daemon-health.svelte.ts, the error from appState.
  import type { DaemonDbMode } from '$lib/health-types.js';

  let { mode, configError = null }: { mode?: DaemonDbMode; configError?: string | null } = $props();
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
{:else if configError}
  <div
    data-component="daemon-status-banner"
    role="status"
    class="flex items-center gap-2 border-b border-warning-soft bg-warning-soft px-4 py-1.5 text-xs text-ink"
  >
    <span>Could not read your settings from the daemon ({configError}) — showing the last ones; trying again.</span>
  </div>
{/if}
