<script lang="ts">
  import type { UnreadProject } from '$lib/fan-out.js';

  /**
   * The projects a cross-project screen could not read, and why (#288). What
   * the screen shows is then known to be partial — a project whose read failed
   * is not passed off as one with nothing to show.
   */
  let { unread }: { unread: UnreadProject[] } = $props();
</script>

{#if unread.length > 0}
  <p
    data-unread-projects={unread.length}
    role="status"
    class="text-sm text-danger bg-danger-soft border border-danger rounded px-3 py-2 mb-4"
  >
    Could not read {unread.length} {unread.length === 1 ? 'project' : 'projects'}:
    {unread.map((u) => `${u.name} (${u.reason})`).join(', ')}
  </p>
{/if}
