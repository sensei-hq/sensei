import { appState } from '$lib/appstate.svelte.js';
import type { LayoutLoad } from './$types.js';

/** Setup wizard needs daemon config + port (loadWizardData reads appState.port). */
export const load: LayoutLoad = async () => {
  if (!appState.loaded) await appState.load();
};
