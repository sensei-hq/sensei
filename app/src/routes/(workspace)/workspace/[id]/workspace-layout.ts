import { error } from '@sveltejs/kit';
import { failureReason, readOutcome, type ApiResult } from '$lib/api.js';

type Ftr = { ftr14d: number | null; ftr14dPrev: number | null; ftrTrend: number[]; sessions7d: number };

/** The reads the workspace window's layout makes — injected so a spec can fail them. */
export interface WorkspaceLayoutApi<P extends { id: string }> {
  tryGetWorkspace: (id: string) => Promise<ApiResult<P>>;
  tryGetWorkspaceFtr: (id: string) => Promise<ApiResult<Ftr>>;
}

/**
 * The workspace window's shell data: its one workspace (not the whole list, every
 * workspace and a folder tree each, to show one). A read that failed is that
 * failure, with its status — not "workspace not found" (#288); a workspace the
 * daemon does not have is a 404. The FTR is carried beside it, or why it could
 * not be read.
 */
export async function loadWorkspaceLayout<P extends { id: string }>(api: WorkspaceLayoutApi<P>, id: string) {
  const [read, ftr] = await Promise.all([api.tryGetWorkspace(id), api.tryGetWorkspaceFtr(id)]);
  if (!read.ok) {
    if (read.error.status === 404) error(404, `Workspace ${id} not found`);
    error(read.error.status || 503, `Could not read the workspace: ${failureReason(read.error)}`);
  }
  const f = readOutcome(ftr);
  return { workspace: read.data, ftrMetrics: f.data, ftrError: f.error, workspaceId: id };
}
