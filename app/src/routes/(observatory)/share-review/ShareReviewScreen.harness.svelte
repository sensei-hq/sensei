<script lang="ts">
  import ShareReviewScreen from './ShareReviewScreen.svelte';
  import type { PublishBatchAction } from './share-review-state.svelte.js';
  import type { ShareReviewBatch } from '$lib/types.js';

  let { batch, actions }: { batch: ShareReviewBatch | null; actions: PublishBatchAction } =
    $props();
</script>

<ShareReviewScreen {batch} {actions} />
