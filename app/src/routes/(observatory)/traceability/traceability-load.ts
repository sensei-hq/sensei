import { failureReason, type ApiResult } from '$lib/api.js';
import { fanOutProjects, type UnreadProject } from '$lib/fan-out.js';
import type { DriftItem } from '$lib/types.js';
import {
  groupDriftByProject,
  rollupDrift,
  toDriftRow,
  type DriftGroup,
  type DriftRollup,
} from './traceability-state.svelte.js';

/** The reads Traceability makes — injected so a spec can fail them. */
export interface DriftApi {
  tryListProjects: () => Promise<ApiResult<Array<{ id: string; name: string }>>>;
  tryGetProjectDrift: (id: string) => Promise<ApiResult<{ items: DriftItem[]; indexedDocs?: number }>>;
}

/**
 * Doc drift across every project (one read per project — no cap: a silent
 * truncation would hide drift on later projects). A list that could not be read
 * is the page's error; a project whose read failed is named in `unread`, never
 * counted as clean (#288).
 */
export async function loadDriftAcross(api: DriftApi): Promise<{
  groups: DriftGroup[] | null;
  rollup: DriftRollup | null;
  unread: UnreadProject[];
  /** Documents indexed across the projects read — null if any did not say (#290). */
  indexedDocs: number | null;
  error: string | null;
}> {
  const list = await api.tryListProjects();
  if (!list.ok) return { groups: null, rollup: null, unread: [], indexedDocs: null, error: failureReason(list.error) };
  const { read, unread } = await fanOutProjects(list.data, (p) => api.tryGetProjectDrift(p.id));
  const rows = read.flatMap(({ project, data }) => data.items.map((it) => toDriftRow(it, project)));
  const docs = read.map(({ data }) => data.indexedDocs);
  const indexedDocs = docs.every((n) => typeof n === 'number') ? docs.reduce<number>((a, n) => a + (n ?? 0), 0) : null;
  return { groups: groupDriftByProject(rows), rollup: rollupDrift(rows), unread, indexedDocs, error: null };
}
