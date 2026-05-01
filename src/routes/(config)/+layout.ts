import { loadWizardData } from '$lib/setup/loaders.js';
import { appState } from '$lib/appstate.svelte.js';

export async function load() {
  return await loadWizardData(appState.port);
}
