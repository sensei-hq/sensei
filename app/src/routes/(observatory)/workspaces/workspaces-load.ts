import type { ApiResult } from '$lib/api.js';
import { failureReason } from '$lib/api.js';
import type { WorkspaceListItem } from '$lib/types.js';
import { baseWorkspace, bucketWorkspaces, workspaceStatus, withQuality } from './buckets.js';

/** The reads the workspaces index makes — injected so a spec can fail them. */
export interface WorkspacesIndexApi {
  tryListWorkspaces: () => Promise<ApiResult<WorkspaceListItem[]>>;
  tryGetWorkspaceFtr: (id: string) => Promise<ApiResult<{ ftr14d: number | null }>>;
  tryGetWorkspaceQualitySignals: (id: string) => Promise<ApiResult<{ open_drift_count: number; ftr_7d: number }>>;
}

/**
 * The workspaces index: the bucketed cards, or why the list could not be read —
 * never an empty index standing in for a failure (#288).
 *
 * The list endpoint carries sessions7d / repos_count / libs_count /
 * last_session_at, so only ACTIVE workspaces need the per-workspace ftr + quality
 * fan-out — dormant and archived cards show neither an FTR stat nor a warning
 * dot. That collapses the fan-out from 2×N to 2×(active few).
 */
export async function loadWorkspacesIndex(api: WorkspacesIndexApi) {
  const list = await api.tryListWorkspaces();
  if (!list.ok) return { buckets: null, error: failureReason(list.error) };
  const cards = await Promise.all(
    list.data.map(async (p) => {
      const base = baseWorkspace(p);
      if (workspaceStatus(base) !== 'active') return base;
      const [ftr, signals] = await Promise.all([api.tryGetWorkspaceFtr(p.id), api.tryGetWorkspaceQualitySignals(p.id)]);
      return withQuality(base, ftr, signals);
    }),
  );
  return { buckets: bucketWorkspaces(cards), error: null };
}
