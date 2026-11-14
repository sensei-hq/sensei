<script lang="ts">
  // One component's preview (#429): its fixtures as tabs (or all of them), in the theme,
  // marks and width the query asks for — the page a person reviews and, for `all`, the
  // one the capture matrix shoots. Nothing here reads the daemon.
  import { setGlyphMode } from '$lib/glyphs';
  import { ALL, fixtureName, previewHref, type PreviewComponent, type PreviewEntry, type PreviewView } from './catalogue.js';

  let data: { component: string; entries: PreviewEntry[]; components: PreviewComponent[]; view: PreviewView } = $props();

  // The marks follow the query string, not the person's preference: a capture must be
  // able to ask for either.
  setGlyphMode(() => data.view.glyph);

  // The theme is the document's `data-mode`, as app.html sets it before first paint;
  // the preview overrides it while the page is open.
  $effect(() => {
    const previous = document.documentElement.dataset.mode;
    document.documentElement.dataset.mode = data.view.theme;
    document.body.dataset.mode = data.view.theme;
    return () => {
      if (previous) {
        document.documentElement.dataset.mode = previous;
        document.body.dataset.mode = previous;
      }
    };
  });

  const shown = $derived(
    data.view.fixture ? data.entries.filter((e) => fixtureName(e) === data.view.fixture) : data.entries,
  );
  const href = (change: Partial<PreviewView>) => previewHref(data.component, data.view, change);

  const CONTROLS: { label: string; key: 'theme' | 'glyph' | 'width'; options: [string, string][] }[] = [
    { label: 'Theme', key: 'theme', options: [['light', 'light'], ['dark', 'dark']] },
    { label: 'Marks', key: 'glyph', options: [['kanji', 'kanji'], ['icon', 'icons']] },
    { label: 'Width', key: 'width', options: [['wide', 'wide'], ['phone', 'phone']] },
  ];
  const PILL = 'rounded-md border border-paper-edge px-2 py-1 text-ink no-underline aria-[current]:bg-paper-mute aria-[current]:border-paper-edge-strong';
</script>

<svelte:head><title>{data.component === ALL ? 'All components' : data.component} · preview</title></svelte:head>

<div class="flex min-h-full flex-wrap bg-paper-soft text-ink md:flex-nowrap" data-preview-page data-preview-ready>
  <nav aria-label="Components" class="w-full shrink-0 border-b border-paper-edge bg-paper px-3 py-3 md:w-52 md:border-b-0 md:border-r">
    <a href="/preview" class="mb-2 block px-2 text-xs text-ink-mute">All components</a>
    <ul class="m-0 flex list-none flex-wrap gap-1 p-0 md:flex-col">
      {#each data.components as c (c.name)}
        <li>
          <a
            href={previewHref(c.name, data.view, { fixture: null })}
            aria-current={c.name === data.component ? 'page' : undefined}
            class="flex justify-between gap-3 rounded-md px-2 py-1 font-mono text-xs text-ink no-underline hover:bg-paper-mute aria-[current]:bg-paper-mute"
          >{c.name}<span class="text-ink-mute tabular-nums">{c.count}</span></a>
        </li>
      {/each}
    </ul>
  </nav>

  <div class="flex min-w-0 flex-1 flex-col">
    <header class="sticky top-0 z-10 flex flex-col gap-3 border-b border-paper-edge bg-paper px-5 py-3">
      <div class="flex flex-wrap items-center gap-x-5 gap-y-2">
        <h1 class="m-0 font-display text-xl font-medium">{data.component === ALL ? 'All components' : data.component}</h1>
        <span class="font-mono text-xs text-ink-mute">{data.entries.length} fixtures · development only</span>
        <div class="ml-auto flex flex-wrap items-center gap-4 text-sm" aria-label="View">
          {#each CONTROLS as control (control.key)}
            <span class="inline-flex items-center gap-1" role="group" aria-label={control.label}>
              {#each control.options as [value, label] (value)}
                <a href={href({ [control.key]: value })} aria-current={data.view[control.key] === value ? 'true' : undefined} class={PILL}>{label}</a>
              {/each}
            </span>
          {/each}
        </div>
      </div>
      {#if data.entries.length > 1}
        <nav aria-label="Fixtures" class="flex flex-wrap gap-1 text-sm" data-preview-tabs>
          <a href={href({ fixture: null })} aria-current={data.view.fixture === null ? 'page' : undefined} class={PILL}>All</a>
          {#each data.entries as e (e.id)}
            <a href={href({ fixture: fixtureName(e) })} aria-current={data.view.fixture === fixtureName(e) ? 'page' : undefined} class="{PILL} font-mono">{fixtureName(e)}</a>
          {/each}
        </nav>
      {/if}
    </header>

    <main class="flex flex-col gap-6 px-5 py-5">
      {#each shown as item (item.id)}
        <section data-preview={item.id} class="flex flex-col gap-2">
          <h2 class="m-0 flex flex-wrap items-baseline gap-x-3 text-sm font-medium">
            <span class="font-mono">{item.id}</span>
            <span class="font-normal text-ink-mute" data-preview-note>{item.note}</span>
          </h2>
          <!-- The sample box is what the capture shoots: the component on the page's paper,
               at the width under review (390px phone, the row's width wide). -->
          <div
            data-preview-sample
            class="rounded-xl border border-paper-edge bg-paper"
            class:p-3={!item.frame}
            class:overflow-hidden={Boolean(item.frame)}
            class:w-[390px]={data.view.width === 'phone'}
            class:max-w-[1100px]={data.view.width === 'wide' && !item.frame}
            class:max-w-[1400px]={data.view.width === 'wide' && Boolean(item.frame)}
            style:height={item.frame ? `${data.view.width === 'phone' ? (item.frame.phoneHeight ?? item.frame.height) : item.frame.height}px` : undefined}
          >
            <item.component {...item.props} />
          </div>
        </section>
      {:else}
        <p class="m-0 text-sm text-ink-mute">No fixture named {data.view.fixture}. <a href={href({ fixture: null })}>Show all of them</a>.</p>
      {/each}
    </main>
  </div>
</div>
