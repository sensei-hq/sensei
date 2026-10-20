<script lang="ts">
  import type { Snippet } from 'svelte';

  // The app's one screen layout: a header that stays in place over a body that
  // is the screen's only scroll owner. A screen's own header, filters or tabs go
  // in `header`; everything that scrolls goes in the body.
  let { header, children }: { header?: Snippet; children: Snippet } = $props();
</script>

<div data-component="screen" class="h-full flex flex-col min-h-0 overflow-hidden">
  {#if header}
    <div data-component="screen-header" class="shrink-0">{@render header()}</div>
  {/if}
  <div data-component="screen-body" class="flex-1 min-h-0 overflow-y-auto">{@render children()}</div>
</div>
