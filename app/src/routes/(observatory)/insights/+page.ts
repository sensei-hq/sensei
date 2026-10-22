import type { PageLoad } from './$types.js';
import { senseiApi, readOutcome } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

/**
 * Load the cross-workspace triage board (unscoped default). The daemon does the
 * bucketing; the client trusts the `column` label on every item. Workspace-scoped
 * refetches happen client-side from the filter chips (see insights-board state).
 * A board that could not be read is carried as its error (#288).
 */
export const load: PageLoad = async () => {
  const { data, error } = readOutcome(await senseiApi(appState.port).tryGetInsights());
  return { board: data, error };
};
