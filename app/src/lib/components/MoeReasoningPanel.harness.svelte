<script lang="ts">
  import MoeReasoningPanel from './MoeReasoningPanel.svelte';
  import type { ImpactReasoning, VerdictTone } from '$lib/impact.js';

  let { reasoning, tone }: {
    reasoning: ImpactReasoning;
    tone?: VerdictTone;
  } = $props();
</script>

<MoeReasoningPanel {reasoning} {tone} />
