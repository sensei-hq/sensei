import type { InsightWorkspaceRef } from '$lib/types.js';

/** Parse `last_session_at` to epoch ms; null / unparseable sorts LAST (-Infinity).
 *  Pure. Mirrors the Workspaces screen's recency ordering. */
function recencyOf(p: InsightWorkspaceRef): number {
  const t = p.last_session_at ? new Date(p.last_session_at).getTime() : NaN;
  return Number.isNaN(t) ? -Infinity : t;
}

/** Pure: the `n` most-recently-active workspaces (last_session_at desc, name asc
 *  tiebreak; never-run workspaces last). The Insights filter shows these as chips
 *  instead of one chip per workspace — the rest are reachable by search. */
export function recentWorkspaces(workspaces: InsightWorkspaceRef[], n = 3): InsightWorkspaceRef[] {
  return [...workspaces]
    .sort((a, b) => recencyOf(b) - recencyOf(a) || a.name.localeCompare(b.name))
    .slice(0, n);
}

/** Pure: the chips to render — the recent `n`, plus the currently-selected
 *  workspace when it isn't already among them, so the active filter is always
 *  visible as a chip (e.g. one picked via search). */
export function chipWorkspaces(
  workspaces: InsightWorkspaceRef[],
  selectedId: string | null,
  n = 3,
): InsightWorkspaceRef[] {
  const recent = recentWorkspaces(workspaces, n);
  if (selectedId == null || recent.some((p) => p.id === selectedId)) return recent;
  const sel = workspaces.find((p) => p.id === selectedId);
  return sel ? [...recent, sel] : recent;
}

/** Pure: workspaces whose name contains `query` (case-insensitive), capped at
 *  `cap`. Empty query → empty (the search dropdown only appears while typing). */
export function matchWorkspaces(
  workspaces: InsightWorkspaceRef[],
  query: string,
  cap = 8,
): InsightWorkspaceRef[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  return workspaces.filter((p) => p.name.toLowerCase().includes(q)).slice(0, cap);
}
