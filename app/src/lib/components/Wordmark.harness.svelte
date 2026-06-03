<script lang="ts">
  import Wordmark from './Wordmark.svelte';
  let { size = 'md' }: { size?: 'sm' | 'md' | 'lg' } = $props();
</script>

<Wordmark {size} />
