<script lang="ts">
  // Test wrapper: provides the glyph mode and the screen-state defaults through
  // context, as each app's root layout does, and renders the shared StateView.
  import { StateView, setGlyphMode, setStateDefaults } from '@sensei/ui';

  let {
    glyphs = 'kanji',
    remedy,
    status,
    error = null,
    onretry,
    kanji = '一',
    title = 'Nothing here',
    description = 'Empty description',
    action,
    notice,
  }: {
    glyphs?: 'kanji' | 'icon';
    remedy?: string;
    status: 'loading' | 'error' | 'empty' | 'ready';
    error?: string | null;
    onretry?: () => void;
    kanji?: string;
    title?: string;
    description?: string;
    action?: Record<string, unknown>;
    notice?: Record<string, unknown>;
  } = $props();
  setGlyphMode(() => glyphs);
  setStateDefaults({ get remedy() { return remedy; } });
</script>

<StateView {status} {error} {onretry} {kanji} {title} {description} action={action as never} notice={notice as never}>
  <div data-testid="ready-content">READY</div>
</StateView>
