import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

// The daemon always returns a full collective-preferences object — the stored
// row, or the defaults when unset. `getCollectivePreferences` falls back to the
// same defaults so a daemon hiccup renders the defaults, never a broken screen.
export const load: PageLoad = async () => {
  const api = senseiApi(appState.port);
  const preferences = await api.getCollectivePreferences();
  return { preferences };
};
