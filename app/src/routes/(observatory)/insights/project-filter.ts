import type { InsightProjectRef } from '$lib/types.js';

/** Parse `last_session_at` to epoch ms; null / unparseable sorts LAST (-Infinity).
 *  Pure. Mirrors the Projects screen's recency ordering. */
function recencyOf(p: InsightProjectRef): number {
  const t = p.last_session_at ? new Date(p.last_session_at).getTime() : NaN;
  return Number.isNaN(t) ? -Infinity : t;
}

/** Pure: the `n` most-recently-active projects (last_session_at desc, name asc
 *  tiebreak; never-run projects last). The Insights filter shows these as chips
 *  instead of one chip per project — the rest are reachable by search. */
export function recentProjects(projects: InsightProjectRef[], n = 3): InsightProjectRef[] {
  return [...projects]
    .sort((a, b) => recencyOf(b) - recencyOf(a) || a.name.localeCompare(b.name))
    .slice(0, n);
}

/** Pure: the chips to render — the recent `n`, plus the currently-selected
 *  project when it isn't already among them, so the active filter is always
 *  visible as a chip (e.g. one picked via search). */
export function chipProjects(
  projects: InsightProjectRef[],
  selectedId: string | null,
  n = 3,
): InsightProjectRef[] {
  const recent = recentProjects(projects, n);
  if (selectedId == null || recent.some((p) => p.id === selectedId)) return recent;
  const sel = projects.find((p) => p.id === selectedId);
  return sel ? [...recent, sel] : recent;
}

/** Pure: projects whose name contains `query` (case-insensitive), capped at
 *  `cap`. Empty query → empty (the search dropdown only appears while typing). */
export function matchProjects(
  projects: InsightProjectRef[],
  query: string,
  cap = 8,
): InsightProjectRef[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  return projects.filter((p) => p.name.toLowerCase().includes(q)).slice(0, cap);
}
