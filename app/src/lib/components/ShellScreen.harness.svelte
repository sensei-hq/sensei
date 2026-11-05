<script lang="ts">
  // Test wrapper: provides the glyph mode and (when given) the view-state store
  // through context, as each app's root layout does, and renders the shared
  // ShellScreen with every band, so each toggle is present.
  import { untrack } from 'svelte';
  import type { PanelState } from '@rokkit/app';
  import { setGlyphMode, setViewStates, ShellScreen } from '@sensei/ui';

  interface ViewStateStore {
    get(viewId: string): PanelState | undefined;
    set(viewId: string, next: PanelState): Promise<void> | void;
    /** Why the store cannot read or keep the views (appState's `viewsError`), or null. */
    error?(): string | null;
  }

  let {
    glyphs = 'kanji',
    store = null,
    viewId = 'test.view',
  }: {
    glyphs?: 'kanji' | 'icon';
    store?: ViewStateStore | null;
    viewId?: string;
  } = $props();
  setGlyphMode(() => glyphs);
  const provided = untrack(() => store);
  if (provided) setViewStates(provided);
</script>

<ShellScreen
  {viewId}
  mark={{ kanji: '構', icon: 'i-test:structure' }}
  title="Structure"
  info={[{ label: 'Use it to', text: 'a' }, { label: 'How to read it', text: 'b' }, { label: 'What to do', text: 'c' }]}
  stats={[{ id: 'modules', title: 'Modules', value: 3 }]}
>
  {#snippet sidebar()}<div data-testid="side">SIDE</div>{/snippet}
  <div data-testid="body">BODY</div>
</ShellScreen>
