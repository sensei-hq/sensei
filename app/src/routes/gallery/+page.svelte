<script lang="ts">
  // The development gallery: every relay component from the catalogue, one
  // section per fixture, in the theme, glyph mode and width the query string
  // asks for — the same page a person reviews and the capture matrix shoots.
  // Nothing here reads the daemon; the components render their fixtures.
  import { setGlyphMode } from '$lib/glyphs';
  import type { PageData } from './$types.js';

  let { data }: { data: PageData } = $props();

  // The glyph mode for every mark below follows the query string, not the
  // person's preference: a capture must be able to ask for either.
  setGlyphMode(() => data.glyph);

  // The theme is the document's `data-mode`, as app.html sets it before first
  // paint; the gallery overrides it for the duration of the page.
  $effect(() => {
    const previous = document.documentElement.dataset.mode;
    document.documentElement.dataset.mode = data.theme;
    document.body.dataset.mode = data.theme;
    return () => {
      if (previous) {
        document.documentElement.dataset.mode = previous;
        document.body.dataset.mode = previous;
      }
    };
  });

  const link = (over: Partial<{ theme: string; glyph: string; width: string }>) => {
    const q = new URLSearchParams({ theme: data.theme, glyph: data.glyph, width: data.width, ...over });
    return `/gallery?${q.toString()}`;
  };
</script>

<svelte:head><title>Relay component gallery</title></svelte:head>

<div class="min-h-full bg-paper-soft text-ink" data-gallery-page data-gallery-ready>
  <header class="sticky top-0 z-10 flex flex-wrap items-center gap-x-5 gap-y-2 border-b border-paper-edge bg-paper px-5 py-3">
    <h1 class="m-0 font-display text-xl font-medium">Relay components</h1>
    <span class="font-mono text-xs text-ink-mute">{data.entries.length} fixtures · development only</span>
    <nav class="ml-auto flex flex-wrap items-center gap-4 text-sm" aria-label="Gallery controls">
      <span class="inline-flex items-center gap-1" role="group" aria-label="Theme">
        <a href={link({ theme: 'light' })} aria-current={data.theme === 'light' ? 'true' : undefined} class="rounded-md border border-paper-edge px-2 py-1 aria-[current]:bg-paper-mute">light</a>
        <a href={link({ theme: 'dark' })} aria-current={data.theme === 'dark' ? 'true' : undefined} class="rounded-md border border-paper-edge px-2 py-1 aria-[current]:bg-paper-mute">dark</a>
      </span>
      <span class="inline-flex items-center gap-1" role="group" aria-label="Marks">
        <a href={link({ glyph: 'kanji' })} aria-current={data.glyph === 'kanji' ? 'true' : undefined} class="rounded-md border border-paper-edge px-2 py-1 aria-[current]:bg-paper-mute">kanji</a>
        <a href={link({ glyph: 'icon' })} aria-current={data.glyph === 'icon' ? 'true' : undefined} class="rounded-md border border-paper-edge px-2 py-1 aria-[current]:bg-paper-mute">icons</a>
      </span>
      <span class="inline-flex items-center gap-1" role="group" aria-label="Width">
        <a href={link({ width: 'wide' })} aria-current={data.width === 'wide' ? 'true' : undefined} class="rounded-md border border-paper-edge px-2 py-1 aria-[current]:bg-paper-mute">wide</a>
        <a href={link({ width: 'phone' })} aria-current={data.width === 'phone' ? 'true' : undefined} class="rounded-md border border-paper-edge px-2 py-1 aria-[current]:bg-paper-mute">phone</a>
      </span>
    </nav>
  </header>

  <main class="flex flex-col gap-6 px-5 py-5">
    {#each data.entries as item (item.id)}
      <section data-gallery={item.id} class="flex flex-col gap-2">
        <h2 class="m-0 flex flex-wrap items-baseline gap-x-3 text-sm font-medium">
          <span class="font-mono">{item.id}</span>
          <span class="text-ink-mute font-normal" data-gallery-note>{item.note}</span>
        </h2>
        <!-- The sample box is what the capture shoots: the component on the page's
             paper, at the width under review (390px phone, the row's width wide). -->
        <div
          data-gallery-sample
          class="rounded-xl border border-paper-edge bg-paper p-3"
          class:w-[390px]={data.width === 'phone'}
          class:max-w-[1100px]={data.width === 'wide'}
        >
          <item.component {...item.props} />
        </div>
      </section>
    {/each}
  </main>
</div>
