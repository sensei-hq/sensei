// The sidebars' one rule for which entry is current (#133 rails, user
// 2026-10-28: the three sidebars are one Rail). Pure, so each rail's nav data
// stays a list and the matching is not copied per rail.

/**
 * The href of the rail entry the current route belongs to: an exact match,
 * else the longest href the path sits under. `skip` names hrefs that never
 * claim a deeper path (the main rail's `/`, which every route sits under).
 */
export function resolveActiveHref(pathname: string, hrefs: string[], skip: string[] = []): string {
  if (hrefs.includes(pathname)) return pathname;
  let best = "";
  for (const href of hrefs) {
    if (skip.includes(href)) continue;
    if (pathname.startsWith(href + "/") && href.length > best.length) best = href;
  }
  return best;
}
