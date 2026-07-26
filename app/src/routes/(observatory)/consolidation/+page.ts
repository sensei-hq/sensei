import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

/**
 * Load the current global consolidated ruleset (approved if present, else the
 * latest proposed). `null` when a consolidation has never run — the page shows
 * the empty state with a "consolidate now" affordance. The daemon owns the
 * approved-vs-proposed selection; the client just renders what it gets.
 */
export const load: PageLoad = async () => {
  const ruleset = await senseiApi(appState.port).getConsolidatedRuleset();
  return { ruleset };
};
