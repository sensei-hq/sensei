import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

// The consolidated overview endpoint assembles the whole landing pane — header,
// top rec, stats, recent sessions — in one call. The (project) layout has
// already resolved appState + the project header for the window chrome, so this
// only needs the one overview fetch. `getProjectOverview` returns the calm
// all-quiet fallback shape on a daemon hiccup, never throws.
export const load: PageLoad = async ({ params }) => {
  const overview = await senseiApi(appState.port).getProjectOverview(params.id);
  return { overview };
};
