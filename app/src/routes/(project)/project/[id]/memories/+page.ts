import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  const api = senseiApi(appState.port);
  const [data, batches] = await Promise.all([
    api.getProjectMemories(params.id),
    // Only proposed batches surface at the top of the page — approved and
    // rejected verdicts stay in an "audit" fold when the user asks for it.
    api.listMemoryShareBatches(params.id, 'proposed'),
  ]);
  return {
    project,
    memories: data.active ?? [],
    total: data.total ?? 0,
    proposedBatches: batches.batches ?? [],
  };
};
