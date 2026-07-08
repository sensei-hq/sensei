<script lang="ts">
  import PatternMini from './PatternMini.svelte';
  import type { PatternCardVm } from './insights-board.svelte.js';

  let { pattern }: { pattern: PatternCardVm } = $props();
</script>

<PatternMini {pattern} />
