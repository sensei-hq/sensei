<script lang="ts">
  import RecCard from './RecCard.svelte';
  import type { RecCardVm } from './insights-board.svelte.js';

  let {
    rec,
    focal = false,
    onApply = () => {},
    onReview = () => {},
    onDismiss = () => {},
  }: {
    rec: RecCardVm;
    focal?: boolean;
    onApply?: () => void;
    onReview?: () => void;
    onDismiss?: () => void;
  } = $props();
</script>

<RecCard {rec} {focal} {onApply} {onReview} {onDismiss} />
