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

  import { ScreenShell, type FilterState, type PanelState, type ScreenShellProps } from '@rokkit/app';
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
    ...rest
  }: ShellProps & { viewId: string } = $props();

  const store = useViewStates();
  const mode = useGlyphMode();
  // Sensei's mode is `kanji | icon`; rokkit draws kanji as `text`.
  const shellGlyphs = $derived(mode() === 'icon' ? 'icon' : 'text');

  // The shell's panels follow the store. They are the wrapper's own state so a
  // refused save can put them back: rokkit flips its copy before we hear of it.
  let panels = $state<PanelState>({});
  $effect(() => {
    const stored = store?.get(viewId);
    if (stored !== undefined) panels = stored;
  });

  let saveError = $state<string | null>(null);
  const readError = $derived(store?.error?.() ?? null);
  const failure = $derived(saveError ?? readError);

  async function keep(next: PanelState) {
    const before = store?.get(viewId) ?? {};
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
<ScreenShell {...rest} {viewId} glyphs={shellGlyphs} {panels} onpanels={keep} bind:filter>
  {#if failure}
    <div class="px-4 pt-4" data-screen-views-notice>
      <Notice tone="fault" size="inline" title={failure} />
    </div>
  {/if}
  {@render children()}
</ScreenShell>
