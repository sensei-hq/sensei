import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { loadWorkspaceMemories } from './memories-load.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { workspace } = await parent();
  return { workspace, ...(await loadWorkspaceMemories(senseiApi(appState.port), params.id)) };
};
