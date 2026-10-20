import type { ApiResult } from '$lib/api.js';
import { failureReason } from '$lib/api.js';
import type { ProjectListItem } from '$lib/types.js';
import { baseProject, bucketProjects, projectStatus, withQuality } from './buckets.js';

/** The reads the projects index makes — injected so a spec can fail them. */
export interface ProjectsIndexApi {
  tryListProjects: () => Promise<ApiResult<ProjectListItem[]>>;
  tryGetProjectFtr: (id: string) => Promise<ApiResult<{ ftr14d: number | null }>>;
  tryGetProjectQualitySignals: (id: string) => Promise<ApiResult<{ open_drift_count: number; ftr_7d: number }>>;
}

/**
 * The projects index: the bucketed cards, or why the list could not be read —
 * never an empty index standing in for a failure (#288).
 *
 * The list endpoint carries sessions7d / repos_count / libs_count /
 * last_session_at, so only ACTIVE projects need the per-project ftr + quality
 * fan-out — dormant and archived cards show neither an FTR stat nor a warning
 * dot. That collapses the fan-out from 2×N to 2×(active few).
 */
export async function loadProjectsIndex(api: ProjectsIndexApi) {
  const list = await api.tryListProjects();
  if (!list.ok) return { buckets: null, error: failureReason(list.error) };
  const cards = await Promise.all(
    list.data.map(async (p) => {
      const base = baseProject(p);
      if (projectStatus(base) !== 'active') return base;
      const [ftr, signals] = await Promise.all([api.tryGetProjectFtr(p.id), api.tryGetProjectQualitySignals(p.id)]);
      return withQuality(base, ftr, signals);
    }),
  );
  return { buckets: bucketProjects(cards), error: null };
}
