<script lang="ts">
  import type { Snippet } from 'svelte';

  // The app's one screen layout: a header that stays in place over a body that
  // is the screen's only scroll owner. A screen's own header, filters or tabs go
  // in `header`; everything that scrolls goes in the body. A list-and-detail
  // screen whose panes scroll on their own sets `fill`: the body then fills the
  // space without scrolling, and each pane owns its scroll.
  // Any other attribute (a `data-screen` test hook, say) lands on the root.
  let {
    header,
    fill = false,
    children,
    ...rest
  }: { header?: Snippet; fill?: boolean; children: Snippet; [attr: `data-${string}`]: unknown } = $props();
</script>

<div {...rest} data-component="screen" class="h-full flex flex-col min-h-0 overflow-hidden">
  {#if header}
    <div data-component="screen-header" class="shrink-0">{@render header()}</div>
  {/if}
  <!-- The scroll body takes focus so the keyboard can scroll it (#245). A fill
       body does not scroll; its panes do. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    data-component="screen-body"
    data-fill={fill || undefined}
    tabindex={fill ? undefined : 0}
    class="flex-1 min-h-0 {fill ? 'flex flex-col overflow-hidden' : 'overflow-y-auto'}"
  >{@render children()}</div>
</div>
