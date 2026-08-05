import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

/**
 * Load the current global consolidated ruleset (approved if present, else the
 * latest proposed). `ruleset: null` means a consolidation has never run — the
 * page shows the empty state with a "consolidate now" affordance. A fetch
 * FAILURE returns `error` instead, so the page shows error-with-Retry rather
 * than the empty state masking a broken daemon (no-fabrication, F8). The daemon
 * owns the approved-vs-proposed selection; the client just renders what it gets.
 */
export const load: PageLoad = async () => {
  const res = await senseiApi(appState.port).tryGetConsolidatedRuleset();
  if (!res.ok) return { ruleset: null, error: res.error.message };
  return { ruleset: res.data, error: null };
};
