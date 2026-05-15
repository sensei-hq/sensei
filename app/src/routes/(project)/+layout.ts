import { appState } from '$lib/appstate.svelte.js';
import type { LayoutLoad } from './$types.js';

/** Project group needs daemon config (active project, etc.). */
export const load: LayoutLoad = async () => {
  if (!appState.loaded) await appState.load();
};
