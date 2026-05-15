import { error } from '@sveltejs/kit';
import { appState } from '$lib/appstate.svelte.js';
import type { LayoutLoad } from './$types.js';

/** Observatory group needs daemon config. Surfacing 503 here means the
 *  observatory never mounts with an empty config silently — the daemon is
 *  expected to be reachable by the time the health gate let the user past. */
export const load: LayoutLoad = async () => {
  if (appState.loaded) return;
  if (!(await appState.load())) {
    throw error(503, 'Daemon is unreachable. Refresh to retry the health check.');
  }
};
