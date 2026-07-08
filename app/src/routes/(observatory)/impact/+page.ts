import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import type { ProjectListItem } from '$lib/types.js';
import { bucketImpact, type ImpactRow } from '$lib/impact.js';

export const load: PageLoad = async () => {
  const api = senseiApi(appState.port);
  const projects = (await api.listProjects()) as ProjectListItem[];

  // Fan out `/api/projects/{id}/impact` across all projects. Each row
  // is a rec that has EITHER a reasoning trace OR a non-pending verdict,
  // so on a fresh install this returns empty and the page shows an
  // EmptyState — that's the correct honest surface.
  const perProject = await Promise.all(
    projects.map(async (p) => {
      const rows = await api.getProjectImpact(p.id);
      return rows.map((r): ImpactRow => ({
        id: r.id,
        projectId: p.id,
        projectName: p.name,
        title: r.title,
        status: r.status,
        actionType: r.actionType ?? null,
        verdict: (r.verdict ?? 'pending') as ImpactRow['verdict'],
        baselineFtr: r.baselineFtr,
        currentFtr:  r.currentFtr,
        ftrDelta:    r.ftrDelta,
        reasoning:   r.reasoning,
      }));
    }),
  );

  const all = perProject.flat();
  return { buckets: bucketImpact(all), total: all.length };
};
