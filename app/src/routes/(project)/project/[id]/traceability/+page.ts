import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { loadProjectDrift } from './drift-load.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  return { project, ...(await loadProjectDrift(senseiApi(appState.port), params.id)) };
};
