import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  const reposData = await senseiApi(appState.port).getProjectRepos(params.id);
  return { project, repos: reposData.repos ?? [] };
};
