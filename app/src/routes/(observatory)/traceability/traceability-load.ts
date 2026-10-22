import { failureReason, type ApiResult } from '$lib/api.js';
import { fanOutWorkspaces, type UnreadWorkspace } from '$lib/fan-out.js';
import type { DriftItem } from '$lib/types.js';
import {
  groupDriftByWorkspace,
  rollupDrift,
  toDriftRow,
  type DriftGroup,
  type DriftRollup,
} from './traceability-state.svelte.js';

/** The reads Traceability makes — injected so a spec can fail them. */
export interface DriftApi {
  tryListWorkspaces: () => Promise<ApiResult<Array<{ id: string; name: string }>>>;
  tryGetWorkspaceDrift: (id: string) => Promise<ApiResult<{ items: DriftItem[]; indexedDocs?: number }>>;
}

/**
 * Doc drift across every workspace (one read per workspace — no cap: a silent
 * truncation would hide drift on later workspaces). A list that could not be read
 * is the page's error; a workspace whose read failed is named in `unread`, never
 * counted as clean (#288).
 */
export async function loadDriftAcross(api: DriftApi): Promise<{
  groups: DriftGroup[] | null;
  rollup: DriftRollup | null;
  unread: UnreadWorkspace[];
  /** Documents indexed across the workspaces read — null if any did not say (#290). */
  indexedDocs: number | null;
  error: string | null;
}> {
  const list = await api.tryListWorkspaces();
  if (!list.ok) return { groups: null, rollup: null, unread: [], indexedDocs: null, error: failureReason(list.error) };
  const { read, unread } = await fanOutWorkspaces(list.data, (p) => api.tryGetWorkspaceDrift(p.id));
  const rows = read.flatMap(({ workspace, data }) => data.items.map((it) => toDriftRow(it, workspace)));
  const docs = read.map(({ data }) => data.indexedDocs);
  const indexedDocs = docs.every((n) => typeof n === 'number') ? docs.reduce<number>((a, n) => a + (n ?? 0), 0) : null;
  return { groups: groupDriftByWorkspace(rows), rollup: rollupDrift(rows), unread, indexedDocs, error: null };
}
