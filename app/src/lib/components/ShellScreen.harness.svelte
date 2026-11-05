<script lang="ts">
  // Test wrapper: provides the glyph mode and (when given) the view-state store
  // through context, as each app's root layout does, and renders the shared
  // ShellScreen with every band, so each toggle is present.
  import { untrack, type Component } from 'svelte';
  import type { PanelState } from '@rokkit/app';
  import * as ui from '@sensei/ui';

  // Phase A (pending: shell-store): the wrapper does not exist yet, so it is
  // reached through the module; phase B imports it by name and drops the cast.
  interface ViewStateStore {
    get(viewId: string): PanelState | undefined;
    set(viewId: string, next: PanelState): Promise<void> | void;
  }
  const pending = ui as unknown as {
    ShellScreen: Component<Record<string, unknown>>;
    setViewStates: (store: ViewStateStore) => void;
  };
  const ShellScreen = pending.ShellScreen;

  let {
    glyphs = 'kanji',
    store = null,
    viewId = 'test.view',
  }: {
    glyphs?: 'kanji' | 'icon';
    store?: ViewStateStore | null;
    viewId?: string;
  } = $props();
  ui.setGlyphMode(() => glyphs);
  const provided = untrack(() => store);
  if (provided) pending.setViewStates(provided);
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
