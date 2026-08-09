import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import type { ProjectListItem } from '$lib/types.js';
import { bucketProjects, isWarning, projectStatus, type EnrichedProject } from './buckets.js';

export const load: PageLoad = async () => {
  const api = senseiApi(appState.port);
  const projects = await api.listProjects();

  // The list endpoint now carries sessions7d / repos_count / libs_count /
  // last_session_at, so only ACTIVE projects need the per-project ftr +
  // quality fanout — dormant/archived cards show neither an FTR stat nor a
  // warning dot, so fetching for them would be wasted work AND would light a
  // spurious amber dot on every dormant project. This collapses the fanout
  // from 2×N to 2×(active few); the rest resolve from the list payload alone.
  const enriched: EnrichedProject[] = await Promise.all(
    (projects as ProjectListItem[]).map(async (p) => {
      const base: EnrichedProject = {
        ...p,
        ftr14d: null,
        sessions7d: p.sessions7d ?? 0,
        warn: false,
        repos_count: p.repos_count ?? 0,
        libs_count: p.libs_count ?? 0,
        last_session_at: p.last_session_at ?? null,
        vision: p.vision ?? null,
      };
      if (projectStatus(base) !== 'active') return base;
      const [ftr, signals] = await Promise.all([
        api.getProjectFtr(p.id),
        api.getProjectQualitySignals(p.id),
      ]);
      return { ...base, ftr14d: ftr.ftr14d, warn: isWarning(signals, base.sessions7d) };
    }),
  );

  return { buckets: bucketProjects(enriched) };
};
