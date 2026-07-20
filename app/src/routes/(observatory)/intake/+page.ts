import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

/** Load the front-door guide (frame + axis prompts + catalog). The state module
 *  owns recommend/confirm thereafter. */
export const load: PageLoad = async () => {
  const guide = await senseiApi(appState.port).getIntakeGuide();
  return { guide };
};
