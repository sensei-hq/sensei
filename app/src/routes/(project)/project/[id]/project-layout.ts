import { error } from '@sveltejs/kit';
import { failureReason, readOutcome, type ApiResult } from '$lib/api.js';

type Ftr = { ftr14d: number | null; ftr14dPrev: number | null; ftrTrend: number[]; sessions7d: number };

/** The reads the project window's layout makes — injected so a spec can fail them. */
export interface ProjectLayoutApi<P extends { id: string }> {
  tryListProjects: () => Promise<ApiResult<P[]>>;
  tryGetProjectFtr: (id: string) => Promise<ApiResult<Ftr>>;
}

/**
 * The project window's shell data. A list that could not be read is that
 * failure, with its status — not "project not found", which is what an empty
 * fallback list made it (#288). A project the daemon does not list is a 404.
 * The FTR is carried beside it, or why it could not be read.
 */
export async function loadProjectLayout<P extends { id: string }>(api: ProjectLayoutApi<P>, id: string) {
  const [list, ftr] = await Promise.all([api.tryListProjects(), api.tryGetProjectFtr(id)]);
  if (!list.ok) error(list.error.status || 503, `Could not read the projects: ${failureReason(list.error)}`);
  const project = list.data.find((p) => p.id === id);
  if (!project) error(404, `Project ${id} not found`);
  const f = readOutcome(ftr);
  return { project, ftrMetrics: f.data, ftrError: f.error, projectId: id };
}
