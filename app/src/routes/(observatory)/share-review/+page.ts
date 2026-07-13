import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

/** Observatory · Share review — load the next approved-but-unsent batch preview.
 *  `getShareReviewBatch` returns `{ batch: null }` when nothing is pending (and
 *  degrades to that on a daemon hiccup), so the screen renders the empty state
 *  rather than a broken one. Publishing re-invalidates this load. */
export const load: PageLoad = async () => {
  const api = senseiApi(appState.port);
  const { batch } = await api.getShareReviewBatch();
  return { batch };
};
