import { error } from '@sveltejs/kit';
import { failureReason, readOutcome, type ApiResult } from '$lib/api.js';

type Ftr = { ftr14d: number | null; ftr14dPrev: number | null; ftrTrend: number[]; sessions7d: number };

/** The reads the workspace window's layout makes — injected so a spec can fail them. */
export interface WorkspaceLayoutApi<P extends { id: string }> {
  tryListWorkspaces: () => Promise<ApiResult<P[]>>;
  tryGetWorkspaceFtr: (id: string) => Promise<ApiResult<Ftr>>;
}

/**
 * The workspace window's shell data. A list that could not be read is that
 * failure, with its status — not "workspace not found", which is what an empty
 * fallback list made it (#288). A workspace the daemon does not list is a 404.
 * The FTR is carried beside it, or why it could not be read.
 */
export async function loadWorkspaceLayout<P extends { id: string }>(api: WorkspaceLayoutApi<P>, id: string) {
  const [list, ftr] = await Promise.all([api.tryListWorkspaces(), api.tryGetWorkspaceFtr(id)]);
  if (!list.ok) error(list.error.status || 503, `Could not read the workspaces: ${failureReason(list.error)}`);
  const workspace = list.data.find((p) => p.id === id);
  if (!workspace) error(404, `Workspace ${id} not found`);
  const f = readOutcome(ftr);
  return { workspace, ftrMetrics: f.data, ftrError: f.error, workspaceId: id };
}
