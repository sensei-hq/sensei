import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { loadProjectMemories } from './memories-load.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  return { project, ...(await loadProjectMemories(senseiApi(appState.port), params.id)) };
};
