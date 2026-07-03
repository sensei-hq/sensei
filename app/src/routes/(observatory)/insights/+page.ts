import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import type { ProjectListItem, Recommendation } from '$lib/types.js';
import { triageBuckets, type TriageRec } from './triage.js';

// The per-project recs endpoint filters by a single status, so we fetch the
// four the triage view cares about in parallel and concat client-side.
// Superseded and low-urgency pending are dropped by `triageBuckets`, so
// there's no point pulling them across the wire.
const STATUSES: TriageRec['status'][] = ['pending', 'accepted', 'dismissed'];

/** Map the per-project wire shape into the client-side triage row.
 *  Defensive on `urgency` because pre-analyzer recs can carry unexpected
 *  values; anything outside high/medium/low falls to `low` so it drops
 *  cleanly. */
function toTriageRec(r: Recommendation, project: ProjectListItem): TriageRec {
  const urgency = (r.urgency ?? 'low') as TriageRec['urgency'];
  const status = (r.status ?? 'pending') as TriageRec['status'];
  return {
    id: r.id,
    projectId: project.id,
    projectName: project.name,
    title: r.title,
    why: r.why ?? r.description ?? '',
    urgency: (['high', 'medium', 'low'] as const).includes(urgency) ? urgency : 'low',
    status,
    actedAt: r.acted_at ?? null,
  };
}

export const load: PageLoad = async () => {
  const api = senseiApi(appState.port);
  const projects = (await api.listProjects()) as ProjectListItem[];

  const perProject = await Promise.all(
    projects.map(async (p) => {
      const byStatus = await Promise.all(
        STATUSES.map((s) => api.getProjectRecommendations(p.id, s)),
      );
      return byStatus.flat().map((r) => toTriageRec(r as Recommendation, p));
    }),
  );

  const all = perProject.flat();
  return { buckets: triageBuckets(all), total: all.length };
};
