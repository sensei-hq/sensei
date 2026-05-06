import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export async function load({ params, parent }: any) {
  const { project, ftrMetrics } = await parent();
  const api = senseiApi(appState.port);
  const [reposData, recs, memoriesData, sessionsData] = await Promise.all([
    api.getProjectRepos(params.id),
    api.getProjectRecommendations(params.id, 'pending'),
    api.getProjectMemories(params.id),
    api.getProjectSessions(params.id, 4),
  ]);
  return {
    project,
    ftrMetrics,
    repos: reposData.repos ?? [],
    topRecommendation: recs[0] ?? null,
    memoryCount: memoriesData.total ?? 0,
    memoriesPendingShare: memoriesData.pendingShare ?? 0,
    recentSessions: sessionsData.sessions ?? [],
  };
}
