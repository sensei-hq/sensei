import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  const recs = await senseiApi(appState.port).getProjectRecommendations(params.id, 'accepted');
  const verdicts = recs ?? [];
  return {
    project,
    verdicts,
    positiveCount: verdicts.filter(r => r.verdict === 'positive').length,
    negativeCount: verdicts.filter(r => r.verdict === 'negative').length,
    pendingCount:  verdicts.filter(r => r.verdict === 'pending').length,
  };
};
