import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { loadWorkspaceImpact } from '$lib/impact-load.js';

// The measured verdicts (`/impact` — not `accepted` recs, where nothing sits)
// and the user-logged impact log, rendered through the observatory's primitives.
export const load: PageLoad = async ({ parent }) => {
  const { workspace } = await parent();
  return { workspace, ...(await loadWorkspaceImpact(senseiApi(appState.port), workspace)) };
};
