<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import ShareReviewScreen from './ShareReviewScreen.svelte';
  import { PublishBatchAction } from './share-review-state.svelte.js';

  let { data } = $props();

  // The Publish controller — api client + reload injected so the screen stays a
  // pure template. A successful publish re-loads via `invalidateAll` (the
  // next-batch refetch drops the sent batch) and the outcome is shown.
  const actions = new PublishBatchAction(senseiApi(appState.port), invalidateAll);
</script>

<ShareReviewScreen batch={data.batch} {actions} loadError={data.error} onretry={invalidateAll} />
