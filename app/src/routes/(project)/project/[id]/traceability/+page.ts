import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  const data = await senseiApi(appState.port).getProjectDrift(params.id);
  return { project, driftItems: data.items ?? [], total: data.total ?? 0, drifted: data.drifted ?? 0, broken: data.broken ?? 0 };
};
