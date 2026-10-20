import { failureReason, readOutcome, type ApiResult } from './api.js';
import { fanOutProjects, type UnreadProject } from './fan-out.js';
import { bucketImpact, toImpactRow, type ImpactBuckets, type ProjectImpactWire } from './impact.js';
import type { ImpactVerdictEntry } from './types.js';

/** The reads the Impact screens make — injected so a spec can fail them. */
export interface ImpactApi {
  tryListProjects: () => Promise<ApiResult<Array<{ id: string; name: string }>>>;
  tryGetProjectImpact: (id: string) => Promise<ApiResult<ProjectImpactWire[]>>;
  tryListImpactVerdicts: (id: string) => Promise<ApiResult<{ verdicts: ImpactVerdictEntry[] }>>;
}

/**
 * The observatory-wide Impact list: every project's measured verdicts. A list
 * that could not be read is the page's error; a project whose read failed is
 * named in `unread` — not counted as a project with nothing measured (#288).
 */
export async function loadImpactAcross(api: ImpactApi): Promise<{
  buckets: ImpactBuckets | null;
  total: number;
  unread: UnreadProject[];
  error: string | null;
}> {
  const list = await api.tryListProjects();
  if (!list.ok) return { buckets: null, total: 0, unread: [], error: failureReason(list.error) };
  const { read, unread } = await fanOutProjects(list.data, (p) => api.tryGetProjectImpact(p.id));
  const rows = read.flatMap(({ project, data }) => data.map((r) => toImpactRow(r, project)));
  return { buckets: bucketImpact(rows), total: rows.length, unread, error: null };
}

/**
 * One project's Impact tab: its measured verdicts (`/impact` — every rec with a
 * reasoning trace or a non-pending verdict) and the user-logged impact log.
 * Either read failing is the page's error (#288).
 */
export async function loadProjectImpact(api: ImpactApi, project: { id: string; name: string }) {
  const [measured, log] = await Promise.all([api.tryGetProjectImpact(project.id), api.tryListImpactVerdicts(project.id)]);
  const m = readOutcome(measured);
  const l = readOutcome(log);
  const rows = (m.data ?? []).map((r) => toImpactRow(r, project));
  return {
    buckets: bucketImpact(rows),
    total: rows.length,
    impactLog: l.data?.verdicts ?? [],
    error: m.error ?? l.error,
  };
}
