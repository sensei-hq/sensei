import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { loadAtlas } from './atlas-load.js';

// A workspace's atlas scopes to the workspace's OWN repos; `?repo=` switches among
// them. The layout already loaded the workspace (name + folders).
export const load: PageLoad = async ({ url, parent }) => {
  const { workspace } = await parent();
  return loadAtlas(senseiApi(appState.port), workspace, url.searchParams.get('repo'));
};
