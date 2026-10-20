import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { loadProjectInstruments } from './instruments-load.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  return { project, ...(await loadProjectInstruments(senseiApi(appState.port), params.id)) };
};
