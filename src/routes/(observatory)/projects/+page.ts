import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async () => {
  const projects = await senseiApi(appState.port).listProjects();
  return { projects: projects ?? [] };
};
