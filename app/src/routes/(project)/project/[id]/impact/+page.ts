import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  const api = senseiApi(appState.port);
  const [recs, impactLog] = await Promise.all([
    api.getProjectRecommendations(params.id, 'accepted'),
    api.listImpactVerdicts(params.id),
  ]);
  const verdicts = recs ?? [];
  return {
    project,
    verdicts,
    positiveCount: verdicts.filter(r => r.verdict === 'positive').length,
    negativeCount: verdicts.filter(r => r.verdict === 'negative').length,
    pendingCount:  verdicts.filter(r => r.verdict === 'pending').length,
    // Manual impact log (T3 Slice 3) — independent lane, user-logged.
    impactLog: impactLog.verdicts ?? [],
  };
};
