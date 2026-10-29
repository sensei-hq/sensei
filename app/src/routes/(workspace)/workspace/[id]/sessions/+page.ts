import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { loadSessionsDigest, sessionsFetcher } from '$lib/sessions-digest.svelte.js';

// The same 7-day window as Observatory Sessions (its shared peer), scoped to
// this workspace. A failed read is carried as its error.
export const load: PageLoad = async ({ params, parent }) => {
  const { workspace } = await parent();
  const digest = await loadSessionsDigest(sessionsFetcher(senseiApi(appState.port), params.id));
  return { workspace, workspaceId: params.id, ...digest };
};
