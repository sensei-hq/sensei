import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { loadAtlas } from './atlas-load.js';

// A project's atlas scopes to the project's OWN repos; `?repo=` switches among
// them. The layout already loaded the project (name + folders).
export const load: PageLoad = async ({ url, parent }) => {
  const { project } = await parent();
  return loadAtlas(senseiApi(appState.port), project, url.searchParams.get('repo'));
};
