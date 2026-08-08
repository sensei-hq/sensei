import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

/** Observatory · Share review — load the next approved-but-unsent batch preview.
 *  `batch: null` means nothing is pending (honest-empty). A fetch FAILURE returns
 *  `error` instead so the screen shows error-with-Retry rather than an empty state
 *  hiding a daemon hiccup (no-fabrication, F8). Publishing re-invalidates this load. */
export const load: PageLoad = async () => {
  const res = await senseiApi(appState.port).tryGetShareReviewBatch();
  if (!res.ok) return { batch: null, error: res.error.message };
  return { batch: res.data.batch, error: null };
};
