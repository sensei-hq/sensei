<script lang="ts">
  import type { CorrectionCardVm } from './insights-board.svelte.js';

  let { correction }: { correction: CorrectionCardVm } = $props();
</script>

<!-- One of the top corrections — a cluster of same-shape mistakes worth a guard. -->
<article
  data-component="correction-mini"
  data-correction-id={correction.id}
  class="flex items-baseline gap-2 rounded bg-paper-soft border border-paper-edge py-2 px-3"
>
  <span class="kanji text-xs text-warning">直</span>
  <span class="flex-1 min-w-0 text-xs text-ink leading-snug truncate">{correction.text}</span>
  <span class="font-mono text-xs text-ink-soft" data-correction-count>{correction.count}×</span>
</article>
