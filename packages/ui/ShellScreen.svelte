<script lang="ts">
  // packages/ui/ShellScreen.svelte — the sensei side of rokkit's ScreenShell
  // (D87, contract §4, §7, §8). Rokkit lays the screen out and persists
  // nothing; this wrapper connects the two host-wide things a screen must not
  // decide for itself:
  //   • the glyph mode — kanji mode draws the mark's kanji, icon mode its icon;
  //   • the view id — the open bands are read and kept in the host's store.
  // A refused save is never dropped: the band goes back to what is stored, and
  // a Notice in the body says why (D83). An unreadable store is said the same
  // way on mount. With no store provided the screen is in memory alone, and
  // nothing is written anywhere.

  import { ScreenShell, type FilterState, type PanelId, type PanelState, type ScreenShellProps } from '@rokkit/app';
  import Notice from './Notice.svelte';
  import { useGlyphMode } from './glyph-mode.js';
  import { useViewStates } from './view-states.js';

  // Everything rokkit takes, but the mode (from the glyph context), the panels
  // (from the store), and the view id (required here).
  type ShellProps = Omit<ScreenShellProps, 'glyphs' | 'panels' | 'onpanels' | 'viewId'>;

  let {
    viewId,
    children,
    filter = $bindable<FilterState>({ values: {}, stat: null }),
    // A screen that draws marks of its own (lane role marks, row marks) reads the
    // mode itself; it passes what it read here, so the title bar and the rows
    // cannot disagree. Absent, the mode comes from the context as usual.
    glyphs: modeGiven,
    // Bands the screen holds open whatever the store remembers — Relay's info
    // band while there is no run is the whole screen (CONTRACT §1b). A pinned
    // band is drawn open, never written as open.
    pinned = [],
    class: className,
    bandAttrs,
    ...rest
  }: ShellProps & { viewId: string; glyphs?: 'kanji' | 'icon'; pinned?: PanelId[] } = $props();

  const store = useViewStates();
  const mode = useGlyphMode();
  // Sensei's mode is `kanji | icon`; rokkit draws kanji as `text`.
  const shellGlyphs = $derived((modeGiven ?? mode()) === 'icon' ? 'icon' : 'text');

  // The shell fills whatever holds it, and its body is the one scroller
  // (contract §1): rokkit's root is `height: 100%`, which against an
  // auto-height wrapper grows and gets clipped — the way Relay lost its scroll.
  const shellClass = $derived(['h-full', 'min-h-0', className].filter(Boolean).join(' '));

  // That makes the sidebar a scroller of its own; one with nothing focusable in it
  // is out of a keyboard's reach (axe scrollable-region-focusable), so it takes
  // focus. A screen's own sidebar attributes still win.
  const shellBandAttrs = $derived<ScreenShellProps['bandAttrs']>({ ...bandAttrs, side: { tabindex: '0', ...bandAttrs?.side } });

  // The shell's panels follow the store. They are the wrapper's own state so a
  // refused save can put them back: rokkit flips its copy before we hear of it.
  let panels = $state<PanelState>({});
  $effect(() => {
    const stored = store?.get(viewId);
    if (stored !== undefined) panels = stored;
  });
  const shownPanels = $derived<PanelState>(
    pinned.length ? { ...panels, ...Object.fromEntries(pinned.map((id) => [id, true])) } : panels,
  );

  let saveError = $state<string | null>(null);
  const readError = $derived(store?.error?.() ?? null);
  const failure = $derived(saveError ?? readError);

  async function keep(shown: PanelState) {
    const before = store?.get(viewId) ?? {};
    // A pinned band is drawn open, not chosen open: keep what was remembered for it.
    const next: PanelState = { ...shown };
    for (const id of pinned) {
      if (id in panels) next[id] = panels[id];
      else delete next[id];
    }
    panels = next;
    if (!store) return;
    try {
      await store.set(viewId, next);
      saveError = null;
    } catch (e) {
      panels = before;
      saveError = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<!-- `sidebar`, `info`, `stats`, `filters`, `footer`, `toolbar` and the rest pass
     through untouched; the body is this screen's children, with the one Notice
     a refused save needs. -->
<ScreenShell {...rest} {viewId} class={shellClass} bandAttrs={shellBandAttrs} glyphs={shellGlyphs} panels={shownPanels} onpanels={keep} bind:filter>
  {#if failure}
    <div class="px-4 pt-4" data-screen-views-notice>
      <Notice tone="fault" size="inline" title={failure} />
    </div>
  {/if}
  {@render children()}
</ScreenShell>
