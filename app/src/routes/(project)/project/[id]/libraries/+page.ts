import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  const api = senseiApi(appState.port);
  const [libsResp, conflictsResp] = await Promise.all([
    api.getProjectLibraries(params.id),
    // T1a signal — version pins that disagree across folders of this project.
    api.getProjectLibraryVersionConflicts(params.id),
  ]);
  const libraries = libsResp.libraries ?? [];
  const conflicts = conflictsResp.conflicts ?? [];
  const wrappedCount = libraries.filter(l => l.hasDocs).length;
  const localCount = libraries.filter(l => l.localSource).length;
  const unwrappedCount = libraries.length - wrappedCount;
  return {
    project,
    libraries,
    conflicts,
    wrappedCount,
    localCount,
    unwrappedCount,
  };
};
