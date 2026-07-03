import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import type { ProjectListItem, Recommendation } from '$lib/types.js';
import { bucketUpgrades, type Upgrade } from './buckets.js';

/** The analyzer's action_type vocabulary that maps to an installable
 *  artifact. Any pending rec whose action_type isn't here is filtered
 *  out at the loader — it's not something the user can "install." */
const INSTALLABLE_ACTION_TYPES: ReadonlySet<string> = new Set([
  'write_skill', 'create_agent', 'revise_rule', 'enrich_memory', 'audit_stale',
]);

function toUpgrade(r: Recommendation, project: ProjectListItem): Upgrade {
  const urgency = (r.urgency ?? 'low') as Upgrade['urgency'];
  return {
    id: r.id,
    projectId: project.id,
    projectName: project.name,
    title: r.title,
    why: r.why ?? r.description ?? '',
    urgency: (['high', 'medium', 'low'] as const).includes(urgency) ? urgency : 'low',
    status: (r.status ?? 'pending') as Upgrade['status'],
    actionType: (r as { action_type?: string; actionType?: string }).action_type
      ?? (r as { actionType?: string }).actionType
      ?? 'unknown',
  };
}

export const load: PageLoad = async () => {
  const api = senseiApi(appState.port);
  const projects = (await api.listProjects()) as ProjectListItem[];

  const perProject = await Promise.all(
    projects.map(async (p) => {
      const recs = await api.getProjectRecommendations(p.id, 'pending');
      return (recs as Recommendation[])
        .map((r) => toUpgrade(r, p))
        .filter((u) => INSTALLABLE_ACTION_TYPES.has(u.actionType));
    }),
  );

  const all = perProject.flat();
  return { buckets: bucketUpgrades(all), total: all.length };
};
