<script lang="ts">
  import { Notice } from '@sensei/ui';
  import { unreadNotice, type UnreadWorkspace } from '$lib/fan-out.js';

  /**
   * The workspaces a cross-workspace screen could not read, and why (#288). What
   * the screen shows is then known to be partial — a workspace whose read failed
   * is not passed off as one with nothing to show.
   */
  let { unread }: { unread: UnreadWorkspace[] } = $props();
</script>

{#if unreadNotice(unread)}
  <div class="mb-4">
    <Notice size="inline" expanded {...unreadNotice(unread)!} data-unread-workspaces={unread.length} />
  </div>
{/if}
