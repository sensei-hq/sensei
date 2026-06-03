<script lang="ts">
  import Spinner from './Spinner.svelte';

  let { size = 10, tone = 'accent' }: {
    size?: number;
    tone?: 'accent' | 'success' | 'ink';
  } = $props();
</script>

<Spinner {size} {tone} />
