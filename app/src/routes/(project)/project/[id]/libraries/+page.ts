import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  const api = senseiApi(appState.port);
  const data = await api.getProjectLibraries(params.id);
  const libraries = data.libraries ?? [];
  const wrappedCount = libraries.filter((l: any) => l.has_instruments).length;
  const unwrappedCount = libraries.length - wrappedCount;
  return { project, libraries, wrappedCount, unwrappedCount };
};
