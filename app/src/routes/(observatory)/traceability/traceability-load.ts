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
  tryGetProjectDrift: (id: string) => Promise<ApiResult<{ items: DriftItem[] }>>;
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
  error: string | null;
}> {
  const list = await api.tryListProjects();
  if (!list.ok) return { groups: null, rollup: null, unread: [], error: failureReason(list.error) };
  const { read, unread } = await fanOutProjects(list.data, (p) => api.tryGetProjectDrift(p.id));
  const rows = read.flatMap(({ project, data }) => data.items.map((it) => toDriftRow(it, project)));
  return { groups: groupDriftByProject(rows), rollup: rollupDrift(rows), unread, error: null };
}
