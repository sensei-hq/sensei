import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { workspace } = await parent();
  const api = senseiApi(appState.port);
  const [libsRes, conflictsRes] = await Promise.all([
    api.tryGetWorkspaceLibraries(params.id),
    // T1a signal — version pins that disagree across folders of this workspace.
    api.tryGetWorkspaceLibraryVersionConflicts(params.id),
  ]);
  // A fetch FAILURE on either call must surface as an error, not an empty
  // library list that hides a broken daemon (no-fabrication, F8). Honest-empty
  // (a workspace with no libraries) still returns error: null with empty arrays.
  if (!libsRes.ok || !conflictsRes.ok) {
    const error =
      (!libsRes.ok && libsRes.error.message) ||
      (!conflictsRes.ok && conflictsRes.error.message) ||
      'Failed to load libraries';
    return { workspace, libraries: [], conflicts: [], wrappedCount: 0, localCount: 0, unwrappedCount: 0, error };
  }
  const libraries = libsRes.data.libraries ?? [];
  const conflicts = conflictsRes.data.conflicts ?? [];
  const wrappedCount = libraries.filter(l => l.hasDocs).length;
  const localCount = libraries.filter(l => l.localSource).length;
  const unwrappedCount = libraries.length - wrappedCount;
  return {
    workspace,
    libraries,
    conflicts,
    wrappedCount,
    localCount,
    unwrappedCount,
    error: null,
  };
};
