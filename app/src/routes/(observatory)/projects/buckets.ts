import type { ProjectListItem } from '$lib/types.js';

/** One row in the segmented Projects list — a project enriched with the
 *  signals shown on its card (FTR%, 7-day sessions, warning dot). */
export interface EnrichedProject extends ProjectListItem {
  ftr14d: number;
  sessions7d: number;
  /** True when a quality signal is red (open drift, or FTR7d below 60%).
   *  Drives the amber dot the mockup shows next to the project name. */
  warn: boolean;
}

/** The three buckets the mockup surfaces. Archived is empty until the
 *  daemon exposes an archive flag on `ProjectListItem` — enumerating it
 *  keeps the UI contract stable when that lands. */
export interface Buckets {
  active:   EnrichedProject[];
  recent:   EnrichedProject[];
  archived: EnrichedProject[];
}

/** Pure: split enriched projects into Active / Recent / Archived and
 *  sort each by the mockup's ordering (Active by sessions7d desc, Recent
 *  by name asc). Any project with sessions in the last 7 days is Active;
 *  the rest are Recent. Archived is reserved for a future flag; today
 *  it's always empty so the section stays discoverable but honest. */
export function bucketProjects(projects: EnrichedProject[]): Buckets {
  const active:   EnrichedProject[] = [];
  const recent:   EnrichedProject[] = [];
  const archived: EnrichedProject[] = [];
  for (const p of projects) {
    if ((p as { archived?: boolean }).archived) {
      archived.push(p);
    } else if (p.sessions7d > 0) {
      active.push(p);
    } else {
      recent.push(p);
    }
  }
  active.sort((a, b) => b.sessions7d - a.sessions7d || a.name.localeCompare(b.name));
  recent.sort((a, b) => a.name.localeCompare(b.name));
  archived.sort((a, b) => a.name.localeCompare(b.name));
  return { active, recent, archived };
}

/** Pure: a project is "warning" if it has open drift OR its 7-day FTR is
 *  below 0.6. Kept small so the bucket rules stay declarative. */
export function isWarning(signals: { open_drift_count: number; ftr_7d: number }): boolean {
  return signals.open_drift_count > 0 || signals.ftr_7d < 0.6;
}
