import { failureReason, readOutcome, type ApiResult } from '$lib/api.js';
import { fanOutWorkspaces, type UnreadWorkspace } from '$lib/fan-out.js';
import type { DojoUpgrade, DojoUpgradesResponse, Recommendation } from '$lib/types.js';
import { bucketUpgrades, type Upgrade, type UpgradeBuckets } from './buckets.js';

/** The reads Upgrades makes — injected so a spec can fail them. */
export interface UpgradesApi {
  tryGetUpgrades: () => Promise<ApiResult<DojoUpgradesResponse>>;
  tryListWorkspaces: () => Promise<ApiResult<Array<{ id: string; name: string }>>>;
  tryGetWorkspaceRecommendations: (id: string, status?: string) => Promise<ApiResult<Recommendation[]>>;
}

/** The analyzer's action_type vocabulary that maps to an installable
 *  artifact. A pending rec whose action_type isn't here is not something the
 *  user can "install", so it is not an upgrade. */
const INSTALLABLE_ACTION_TYPES: ReadonlySet<string> = new Set([
  'write_skill', 'create_agent', 'revise_rule', 'enrich_memory', 'audit_stale',
]);

export function toUpgrade(r: Recommendation, workspace: { id: string; name: string }): Upgrade {
  const urgency = (r.urgency ?? 'low') as Upgrade['urgency'];
  return {
    id: r.id,
    workspaceId: workspace.id,
    workspaceName: workspace.name,
    title: r.title,
    why: r.why ?? r.description ?? '',
    urgency: (['high', 'medium', 'low'] as const).includes(urgency) ? urgency : 'low',
    status: (r.status ?? 'pending') as Upgrade['status'],
    actionType: (r as { action_type?: string; actionType?: string }).action_type
      ?? (r as { actionType?: string }).actionType
      ?? 'unknown',
  };
}

/**
 * Both Upgrades lanes, each the daemon's read or why it failed (#288):
 *  • the Dōjō downstream inbox — independent of the local recommendations, so
 *    read alongside them, and its failure is its lane's alone;
 *  • every workspace's pending, installable recommendations — a list that could
 *    not be read is the lane's error; a workspace whose read failed is named.
 */
export async function loadUpgrades(api: UpgradesApi): Promise<{
  dojoUpgrades: DojoUpgrade[];
  dojoUnread: number;
  dojoError: string | null;
  buckets: UpgradeBuckets | null;
  total: number;
  unread: UnreadWorkspace[];
  localError: string | null;
}> {
  const [dojoRead, list] = await Promise.all([api.tryGetUpgrades(), api.tryListWorkspaces()]);
  const dojo = readOutcome(dojoRead);
  const lane = { dojoUpgrades: dojo.data?.artifacts ?? [], dojoUnread: dojo.data?.unread_count ?? 0, dojoError: dojo.error };
  if (!list.ok) return { ...lane, buckets: null, total: 0, unread: [], localError: failureReason(list.error) };
  const { read, unread } = await fanOutWorkspaces(list.data, (p) => api.tryGetWorkspaceRecommendations(p.id, 'pending'));
  const upgrades = read.flatMap(({ workspace, data }) =>
    data.map((r) => toUpgrade(r, workspace)).filter((u) => INSTALLABLE_ACTION_TYPES.has(u.actionType)),
  );
  return { ...lane, buckets: bucketUpgrades(upgrades), total: upgrades.length, unread, localError: null };
}
