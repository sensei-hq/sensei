<script lang="ts">
  import CorrectionMini from './CorrectionMini.svelte';
  import type { CorrectionCardVm } from './insights-board.svelte.js';

  let { correction }: { correction: CorrectionCardVm } = $props();
</script>

<CorrectionMini {correction} />
