import { appState } from '$lib/appstate.svelte.js';
import type { LayoutLoad } from './$types.js';

/** Setup wizard layout. `appState.load()` always succeeds (falls back to
 *  defaults on daemon failure); routing is hooks::reroute's job. */
export const load: LayoutLoad = async () => {
  if (appState.loaded) return;
  await appState.load();
};
