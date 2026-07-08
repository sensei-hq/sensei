import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { bucketImpact, type ImpactRow } from '$lib/impact.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  const api = senseiApi(appState.port);

  // Primary source for the verdicts panel — the analyzer's measured impact
  // reports (`GET /api/projects/{id}/impact`): every rec that carries a
  // reasoning trace OR a non-pending verdict. `getProjectRecommendations(id,
  // 'accepted')` was the wrong source — nothing sits in `accepted` status, so
  // the panel rendered empty even though measured verdicts exist.
  const [measured, impactLog] = await Promise.all([
    api.getProjectImpact(params.id),
    api.listImpactVerdicts(params.id),
  ]);

  // Map onto the shared `ImpactRow` shape so this tab renders through the same
  // primitives as the observatory-wide Impact screen. project/name come from
  // this project — the tab hides the project column since it's implied.
  const verdicts: ImpactRow[] = measured.map((r) => ({
    id: r.id,
    projectId: params.id,
    projectName: project?.name ?? '',
    title: r.title,
    status: r.status,
    actionType: r.actionType ?? null,
    verdict: (r.verdict ?? 'pending') as ImpactRow['verdict'],
    baselineFtr: r.baselineFtr,
    currentFtr: r.currentFtr,
    ftrDelta: r.ftrDelta,
    reasoning: r.reasoning,
  }));

  const buckets = bucketImpact(verdicts);

  return {
    project,
    buckets,
    total: verdicts.length,
    // Manual impact log (T3 Slice 3) — independent lane, user-logged.
    impactLog: impactLog.verdicts ?? [],
  };
};
