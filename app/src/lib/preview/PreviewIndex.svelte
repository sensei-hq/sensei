<script lang="ts">
  // The development preview's index (#429): every component, each a link to its own
  // page, and one to all of them at once.
  import { ALL, previewHref, readView, type PreviewComponent } from './catalogue.js';

  let { components }: { components: PreviewComponent[] } = $props();
  const view = readView(new URLSearchParams());
  const total = $derived(components.reduce((n, c) => n + c.count, 0));
</script>

<svelte:head><title>Component preview</title></svelte:head>

<div class="min-h-full bg-paper-soft text-ink" data-preview-index>
  <header class="flex flex-wrap items-baseline gap-x-4 gap-y-1 border-b border-paper-edge bg-paper px-5 py-4">
    <h1 class="m-0 font-display text-xl font-medium">Component preview</h1>
    <span class="font-mono text-xs text-ink-mute">{components.length} components · {total} fixtures · development only</span>
    <a class="ml-auto text-sm text-ink" href={previewHref(ALL, view)} data-preview-all>Everything on one page</a>
  </header>
  <ul class="m-0 grid list-none gap-3 p-5 [grid-template-columns:repeat(auto-fill,minmax(12rem,1fr))]">
    {#each components as c (c.name)}
      <li>
        <a
          href={previewHref(c.name, view)}
          data-preview-component={c.name}
          class="flex items-baseline justify-between gap-3 rounded-[10px] border border-paper-edge bg-paper px-3.5 py-3 text-ink no-underline hover:bg-paper-mute"
        >
          <span class="font-mono text-sm">{c.name}</span>
          <span class="text-xs text-ink-mute tabular-nums">{c.count}</span>
        </a>
      </li>
    {/each}
  </ul>
</div>
