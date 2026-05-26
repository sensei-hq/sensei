import { error } from '@sveltejs/kit';
import { appState } from '$lib/appstate.svelte.js';
import type { LayoutLoad } from './$types.js';

export const load: LayoutLoad = async () => {
  if (appState.loaded) return;
  if (!(await appState.load())) {
    throw error(503, 'Daemon is unreachable. Refresh to retry the health check.');
  }
};
