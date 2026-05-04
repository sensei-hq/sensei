import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project, ftrMetrics } = await parent();
  const data = await senseiApi(appState.port).getProjectSessions(params.id, 50);
  return { project, ftrMetrics, sessions: data.sessions ?? [] };
};
