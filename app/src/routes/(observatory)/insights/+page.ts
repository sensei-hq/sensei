import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

/**
 * Load the cross-project triage board (unscoped default). The daemon does the
 * bucketing; the client trusts the `column` label on every item. Project-scoped
 * refetches happen client-side from the filter chips (see insights-board state).
 */
export const load: PageLoad = async () => {
  const board = await senseiApi(appState.port).getInsights();
  return { board };
};
