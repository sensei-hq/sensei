// Pure wire→kit mapper for the org Workspaces screen (dojo `/org/[slug]/workspaces`).
// Projects the `client-data.ts` `WorkspaceRow` (a `dojo.workspaces` row) onto the
// presentational `KitWorkspace` the shipped `ScrWorkspaces` declares — so the screen
// renders real data with no component change. `lastRun` is the relative age of
// the last run (or "—" when never run). Fields dojo.workspaces doesn't carry
// (needs-attention count, the maintainer note) are honest defaults, not fabricated
// — the daemon populates the row; those signals wire later.
import type { WorkspaceRow } from './client-data';
import { relativeAge } from './triage/view';
import type { KitWorkspace } from './components/kit/types';

/** WorkspaceRow → KitWorkspace. Pure; `now` injected for a deterministic age. */
export function toKitWorkspace(p: WorkspaceRow, now: Date = new Date()): KitWorkspace {
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

/** WorkspaceRow[] → KitWorkspace[], preserving the API's order. Pure. */
export function toKitWorkspaces(workspaces: WorkspaceRow[], now: Date = new Date()): KitWorkspace[] {
	return workspaces.map((p) => toKitWorkspace(p, now));
}
