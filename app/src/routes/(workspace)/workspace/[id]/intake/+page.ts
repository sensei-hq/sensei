import type { PageLoad } from './$types.js';
import { senseiApi, readOutcome } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

/** Load the front-door guide (frame + axis prompts + catalog). The guide is
 *  global; the run it starts is workspace-scoped, so we hand the state the workspace
 *  id from the route. The state module owns recommend/confirm thereafter. A
 *  guide that could not be read is carried as its error (#288). */
export const load: PageLoad = async ({ params }) => {
  const { data, error } = readOutcome(await senseiApi(appState.port).tryGetIntakeGuide());
  return { guide: data, guideError: error, workspaceId: params.id };
};
