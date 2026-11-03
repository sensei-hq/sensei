<script lang="ts">
  import type { UnreadWorkspace } from '$lib/fan-out.js';

  /**
   * The workspaces a cross-workspace screen could not read, and why (#288). What
   * the screen shows is then known to be partial — a workspace whose read failed
   * is not passed off as one with nothing to show.
   */
  let { unread }: { unread: UnreadWorkspace[] } = $props();
</script>

{#if unread.length > 0}
  <p
    data-unread-workspaces={unread.length}
    role="status"
    class="text-sm text-on-danger-soft bg-danger-soft border border-danger rounded px-3 py-2 mb-4"
  >
    Could not read {unread.length} {unread.length === 1 ? 'workspace' : 'workspaces'}:
    {unread.map((u) => `${u.name} (${u.reason})`).join(', ')}
  </p>
{/if}
