<script lang="ts">
  import ScreenState from './ScreenState.svelte';

  let {
    status,
    error = null,
    onretry,
    kanji = '一',
    title = 'Nothing here',
    description = 'Empty description',
    notice,
  }: {
    status: 'loading' | 'error' | 'empty' | 'ready';
    error?: string | null;
    onretry?: () => void;
    kanji?: string;
    title?: string;
    description?: string;
    notice?: Record<string, unknown>;
  } = $props();
</script>

<ScreenState {status} {error} {onretry} {kanji} {title} {description} notice={notice as never}>
  <div data-testid="ready-content">READY</div>
</ScreenState>
