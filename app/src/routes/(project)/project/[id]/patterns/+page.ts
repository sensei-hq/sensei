import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  const data = await senseiApi(appState.port).getProjectPatterns(params.id);
  return { project, followed: data.followed ?? [], antiPatterns: data.antiPatterns ?? [] };
};
