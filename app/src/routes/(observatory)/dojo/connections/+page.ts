import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

// The daemon returns a top-level array of connections (empty when no Dōjō is
// paired — the honest empty state today). `getDojoMemberships` falls back to
// `[]` so a daemon hiccup renders the empty state, never a broken screen.
export const load: PageLoad = async () => {
  const api = senseiApi(appState.port);
  const connections = await api.getDojoMemberships();
  return { connections };
};
