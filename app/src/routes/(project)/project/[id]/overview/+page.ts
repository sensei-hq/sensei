import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project, ftrMetrics } = await parent();
  const api = senseiApi(appState.port);
  const [reposData, recs, memoriesData, sessionsData, ftrDaily, hotspots, signals, teachings] = await Promise.all([
    api.getProjectRepos(params.id),
    api.getProjectRecommendations(params.id, 'pending'),
    api.getProjectMemories(params.id),
    api.getProjectSessions(params.id, 4),
    api.getProjectFtrDaily(params.id, 14),
    api.getProjectHotspots(params.id, 7),
    api.getProjectQualitySignals(params.id),
    api.getProjectTeachings(params.id, 5),
  ]);
  return {
    project,
    ftrMetrics,
    repos: reposData.repos ?? [],
    topRecommendation: (recs as any[])[0] ?? null,
    memoryCount: memoriesData.total ?? 0,
    memoriesPendingShare: memoriesData.pendingShare ?? 0,
    recentSessions: sessionsData.sessions ?? [],
    ftrDaily: ftrDaily.ftr_daily ?? [],
    hotspots: hotspots.hotspots ?? [],
    qualitySignals: signals,
    teachings: teachings.teachings ?? [],
  };
};
