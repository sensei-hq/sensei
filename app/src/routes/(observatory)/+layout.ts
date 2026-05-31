import { appState } from '$lib/appstate.svelte.js';
import type { LayoutLoad } from './$types.js';

/** Observatory group: hydrate the appState cache. `load()` falls back to
 *  empty defaults if the daemon is unreachable — routing is hooks::reroute's
 *  job, so we don't translate cache failures into 503s here. */
export const load: LayoutLoad = async () => {
  if (appState.loaded) return;
  await appState.load();
};
