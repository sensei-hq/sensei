// Pure wire→kit mappers for the personal Contributions screen (F5). Turn the
// federated dōjō rows — `dojo.artifacts` the user contributed (mine) and
// `dojo.downstream_inbox ⋈ dojo.artifacts` approved for them (downstream) — into
// the `KitContribution[]`/`KitDownstream[]` the shipped `ScrContributions`
// renders. Side-effect-free so the mapping unit-tests without a DOM or a Worker.
//
// Access is user/membership-primary: "mine" = artifacts `contributed_by` the
// user (the contribute pipeline stamps it server-side); "downstream" = the
// distribution ledger rows for the user's memberships. Honest-empty until the
// pipeline federates — never a fabricated row.

import type { KitContribution, KitDownstream } from './components/kit/types';
import { relativeAge } from './triage/view';

/** artifact_kind → its brand glyph (mirrors the enum's doc glyphs). Unknown →
 *  the neutral 共 (share) glyph, never a wrong intent. */
const KIND_KANJI: Record<string, string> = {
	principle: '理',
	pattern: '紋',
	prompt: '問',
	guard: '守',
	skill: '技',
	agent: '装'
};
export function kindKanji(kind: string): string {
	return KIND_KANJI[kind] ?? '共';
}

/** dojo.artifact_status → the screen's status vocabulary (drives the chip):
 *  submitted → pending (in triage), published → approved, archived → declined. */
export function artifactStatus(status: string): string {
	if (status === 'published') return 'approved';
	if (status === 'archived') return 'declined';
	return 'pending';
}

/** A row is adopted downstream when its inbox state is applied or pinned
 *  (muted/pending are not adoptions). */
export function isAdopted(state: string): boolean {
	return state === 'applied' || state === 'pinned';
}

/** A short scope label from the artifact's scope jsonb, if it carries a display
 *  hint; otherwise empty (honest — no fabricated scope). */
export function scopeLabel(scope: Record<string, unknown> | null | undefined): string {
	if (!scope) return '';
	const level = typeof scope.level === 'string' ? scope.level : '';
	const name = typeof scope.name === 'string' ? scope.name : '';
	return [level, name].filter(Boolean).join(' · ');
}

/** One `dojo.artifacts` row the user contributed, as the read route returns it
 *  (tenant name embedded as `dest`). */
export interface ContributionRow {
	kind: string;
	title: string;
	status: string;
	attribution: { mode?: string } | null;
	scope: Record<string, unknown> | null;
	created_at: string;
	dest: string | null;
}

/** One `downstream_inbox ⋈ artifacts` row approved for the user (origin tenant
 *  name embedded as `from`). `id` is the artifact id — the key the Pin/adopt
 *  write targets. */
export interface DownstreamRow {
	id: string;
	state: string;
	created_at: string;
	kind: string;
	title: string;
	scope: Record<string, unknown> | null;
	from: string | null;
}

/** ContributionRow → KitContribution. `anonymous` is data-driven from
 *  `attribution.mode` (Rule B: credit-only, dereference is always-on). Pure. */
export function toKitContribution(r: ContributionRow, now: Date = new Date()): KitContribution {
	return {
		kanji: kindKanji(r.kind),
		title: r.title,
		dest: r.dest ?? '—',
		scope: scopeLabel(r.scope),
		status: artifactStatus(r.status),
		when: relativeAge(r.created_at, now),
		note: '',
		anonymous: r.attribution?.mode === 'anonymous'
	};
}

export function toKitContributions(rows: ContributionRow[], now: Date = new Date()): KitContribution[] {
	return rows.map((r) => toKitContribution(r, now));
}

/** DownstreamRow → KitDownstream. Pure. */
export function toKitDownstream(r: DownstreamRow, now: Date = new Date()): KitDownstream {
	return {
		id: r.id,
		kanji: kindKanji(r.kind),
		title: r.title,
		from: r.from ?? '—',
		scope: scopeLabel(r.scope),
		when: relativeAge(r.created_at, now),
		adopted: isAdopted(r.state),
		kind: r.kind
	};
}

export function toKitDownstreams(rows: DownstreamRow[], now: Date = new Date()): KitDownstream[] {
	return rows.map((r) => toKitDownstream(r, now));
}
