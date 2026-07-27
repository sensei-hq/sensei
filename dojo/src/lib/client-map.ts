// Pure wire→kit mapper for the org lead Engagements screen (dojo
// `/org/[slug]/engagements`). Takes the `client-data.ts` `Engagement` wire type
// (the SAME row the shipped `(console)` engagements screen fetches from GET
// …/engagements) and projects it onto the presentational `KitEngagement` the
// dojo ScrEngagements already declares. Side-effect-free so it's DRY + unit-
// tested once; `now` is injected for a deterministic "since".
//
// On lessons/dropped: the engagements LIST route carries no kept-vs-dropped
// artifact counts — those live on GET …/audit/artifacts (per engagement), a
// separate call the console engagements list does not make. Rather than fake a
// number, the mapper leaves both 0 (the row reads "0 lessons kept · 0 stripped"
// honestly); wiring the audit counts is a follow-on when that panel is real.

import type { Engagement } from './client-data';
import { relativeAge } from './triage-view';
import type { KitEngagement } from './components/kit/types';

/** The client-engagement glyph (客 — "guest/client"), matching the fixture. */
const CLIENT_KANJI = '客';

/** One `{ project_id, name }` binding off the jsonb array (defensively typed). */
interface ProjectBinding {
	name?: unknown;
	project_id?: unknown;
}

/**
 * The engagement's projects as a display string — the bound project names joined
 * with " · ", falling back to the project_id, then to "—" when the binding array
 * is empty/malformed. `project_bindings` is jsonb (`unknown` on the wire).
 */
export function bindingsLabel(projectBindings: unknown): string {
	if (!Array.isArray(projectBindings)) return '—';
	const names = (projectBindings as ProjectBinding[])
		.map((b) => (typeof b?.name === 'string' ? b.name : typeof b?.project_id === 'string' ? b.project_id : ''))
		.filter((s) => s.length > 0);
	return names.length ? names.join(' · ') : '—';
}

/**
 * Engagement → KitEngagement. The kit row wants: id, kanji, client, projects,
 * lessons, dropped, since, status. `since` is how long the engagement has run —
 * from `starts_on` when set, else the `created_at` — as a compact relative age.
 * `lessons`/`dropped` are 0 (the list route carries no audit counts; see the
 * module note). Pure.
 */
export function toKitEngagement(e: Engagement, now: Date = new Date()): KitEngagement {
	return {
		id: e.id,
		kanji: CLIENT_KANJI,
		client: e.client,
		projects: bindingsLabel(e.project_bindings),
		lessons: 0,
		dropped: 0,
		since: relativeAge(e.starts_on ?? e.created_at, now),
		status: e.status
	};
}

/** Engagement[] → KitEngagement[], preserving the API's order. Pure. */
export function toKitEngagements(engagements: Engagement[], now: Date = new Date()): KitEngagement[] {
	return engagements.map((e) => toKitEngagement(e, now));
}
