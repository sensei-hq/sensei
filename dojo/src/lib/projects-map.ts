// Pure wire→kit mapper for the org Projects screen (dojo `/org/[slug]/projects`).
// Projects the `client-data.ts` `ProjectRow` (a `dojo.projects` row) onto the
// presentational `KitProject` the shipped `ScrProjects` declares — so the screen
// renders real data with no component change. `lastRun` is the relative age of
// the last run (or "—" when never run). Fields dojo.projects doesn't carry
// (needs-attention count, the maintainer note) are honest defaults, not fabricated
// — the daemon populates the row; those signals wire later.
import type { ProjectRow } from './client-data';
import { relativeAge } from './triage/view';
import type { KitProject } from './components/kit/types';

/** ProjectRow → KitProject. Pure; `now` injected for a deterministic age. */
export function toKitProject(p: ProjectRow, now: Date = new Date()): KitProject {
	return {
		id: p.id,
		name: p.name,
		repo: p.slug,
		classification: p.classification,
		phase: p.phase,
		lastRun: p.last_run_at ? relativeAge(p.last_run_at, now) : '—',
		runsWeek: p.runs_week,
		needs: 0,
		dojoName: null,
		note: ''
	};
}

/** ProjectRow[] → KitProject[], preserving the API's order. Pure. */
export function toKitProjects(projects: ProjectRow[], now: Date = new Date()): KitProject[] {
	return projects.map((p) => toKitProject(p, now));
}
