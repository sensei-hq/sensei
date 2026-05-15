import { appState } from '$lib/appstate.svelte.js';
import type { LayoutLoad } from './$types.js';

/** Observatory group needs daemon config (active project, sidebar prefs, …). */
export const load: LayoutLoad = async () => {
  if (!appState.loaded) await appState.load();
};
