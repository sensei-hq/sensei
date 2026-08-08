<script lang="ts">
  import ScreenState from './ScreenState.svelte';

  let {
    status,
    error = null,
    onretry,
    kanji = '一',
    title = 'Nothing here',
    description = 'Empty description',
  }: {
    status: 'loading' | 'error' | 'empty' | 'ready';
    error?: string | null;
    onretry?: () => void;
    kanji?: string;
    title?: string;
    description?: string;
  } = $props();
</script>

<ScreenState {status} {error} {onretry} {kanji} {title} {description}>
  <div data-testid="ready-content">READY</div>
</ScreenState>
