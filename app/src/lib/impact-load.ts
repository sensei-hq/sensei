import { failureReason, readOutcome, type ApiResult } from './api.js';
import { fanOutWorkspaces, type UnreadWorkspace } from './fan-out.js';
import { bucketImpact, toImpactRow, type ImpactBuckets, type WorkspaceImpactWire } from './impact.js';
import type { ImpactVerdictEntry } from './types.js';

/** The reads the Impact screens make — injected so a spec can fail them. */
export interface ImpactApi {
  tryListWorkspaces: () => Promise<ApiResult<Array<{ id: string; name: string }>>>;
  tryGetWorkspaceImpact: (id: string) => Promise<ApiResult<WorkspaceImpactWire[]>>;
  tryListImpactVerdicts: (id: string) => Promise<ApiResult<{ verdicts: ImpactVerdictEntry[] }>>;
}

/**
 * The observatory-wide Impact list: every workspace's measured verdicts. A list
 * that could not be read is the page's error; a workspace whose read failed is
 * named in `unread` — not counted as a workspace with nothing measured (#288).
 */
export async function loadImpactAcross(api: ImpactApi): Promise<{
  buckets: ImpactBuckets | null;
  total: number;
  unread: UnreadWorkspace[];
  error: string | null;
}> {
  const list = await api.tryListWorkspaces();
  if (!list.ok) return { buckets: null, total: 0, unread: [], error: failureReason(list.error) };
  const { read, unread } = await fanOutWorkspaces(list.data, (p) => api.tryGetWorkspaceImpact(p.id));
  const rows = read.flatMap(({ workspace, data }) => data.map((r) => toImpactRow(r, workspace)));
  return { buckets: bucketImpact(rows), total: rows.length, unread, error: null };
}

/**
 * One workspace's Impact tab: its measured verdicts (`/impact` — every rec with a
 * reasoning trace or a non-pending verdict) and the user-logged impact log.
 * Either read failing is the page's error (#288).
 */
export async function loadWorkspaceImpact(api: ImpactApi, workspace: { id: string; name: string }) {
  const [measured, log] = await Promise.all([api.tryGetWorkspaceImpact(workspace.id), api.tryListImpactVerdicts(workspace.id)]);
  const m = readOutcome(measured);
  const l = readOutcome(log);
  const rows = (m.data ?? []).map((r) => toImpactRow(r, workspace));
  return {
    buckets: bucketImpact(rows),
    total: rows.length,
    impactLog: l.data?.verdicts ?? [],
    error: m.error ?? l.error,
  };
}
