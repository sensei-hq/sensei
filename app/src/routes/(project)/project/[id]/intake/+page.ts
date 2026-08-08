import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

/** Load the front-door guide (frame + axis prompts + catalog). The guide is
 *  global; the run it starts is project-scoped, so we hand the state the project
 *  id from the route. The state module owns recommend/confirm thereafter. */
export const load: PageLoad = async ({ params }) => {
  const guide = await senseiApi(appState.port).getIntakeGuide();
  return { guide, projectId: params.id };
};
