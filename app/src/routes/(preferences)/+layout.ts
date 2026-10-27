import { appState } from '$lib/appstate.svelte.js';
import type { LayoutLoad } from './$types.js';

/** The settings window (user, 2026-10-27: a child window, as a Mac app's
 *  preferences are): hydrate the appState cache, as the observatory does. */
export const load: LayoutLoad = async () => {
  if (appState.loaded) return;
  await appState.load();
};
