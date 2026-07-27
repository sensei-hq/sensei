// Pure wire→kit mappers for the org maintainer Triage + Approvals screens (dojo
// `/org/[slug]/triage` · `/approvals`). Takes the `triage-data.ts` `TriageRow`
// wire type (the SAME row the shipped `(console)` maintainer screen fetches from
// GET …/triage) and projects it onto the presentational `KitTriageGroup[]` /
// `KitApproval[]` / `KitCandidateDetail` the dojo screens already declare.
//
// Side-effect-free so it's DRY + unit-tested once; the existing pure triage-view
// helpers (`groupByScope` · `scopeLabel` · `kindKanji` · `relativeAge`) are
// reused rather than reimplemented.
//
// On impact/detail: the triage LIST row carries no impact classification or the
// rich candidate detail (learning · cause · evidence · conflict) — those live on
// the artifact/cluster, a separate read the list route doesn't make. Rather than
// fake them, impact is derived from confidence (a high-confidence candidate reads
// `high`), and a candidate's detail is a best-effort projection of the row's own
// fields (a per-candidate detail lookup is a follow-on when that route is real).

import type { TriageRow } from './triage-data';
import { groupByScope, scopeLabel, kindKanji, relativeAge } from './triage/view';
import type {
	KitTriageGroup,
	KitTriageCandidate,
	KitApproval,
	KitCandidateDetail
} from './components/kit/types';

/**
 * The impact band for a candidate from its confidence: ≥0.90 reads `high`
 * (routes to a second approval), else `normal`. The list route carries no
 * explicit safety/impact tag, so this is the honest signal available; a real
 * per-artifact impact lands with the cluster read.
 */
export function impactForConfidence(confidence: number | null): string {
	if (confidence != null && confidence >= 0.9) return 'high';
	return 'normal';
}

/** A single origin line for a candidate: contributor count + relative age. */
function originLine(row: TriageRow, now: Date): string {
	const contribs = `${row.contributor_count} contributor${row.contributor_count === 1 ? '' : 's'}`;
	const age = relativeAge(row.created_at, now);
	return age ? `${contribs} · ${age}` : contribs;
}

/** TriageRow → KitTriageCandidate. `conflicts` is 0 (the list route carries no
 *  ladder-conflict count); `dups` is 1 when the row has a nearest artifact above
 *  the flag band (similarity ≥ 0.75), else 0. Pure. */
export function toKitCandidate(row: TriageRow, now: Date = new Date()): KitTriageCandidate {
	const dups = row.similarity != null && row.similarity >= 0.75 && row.nearest_artifact_id ? 1 : 0;
	return {
		id: row.signature,
		kanji: kindKanji(row.kind),
		title: row.title,
		origin: originLine(row, now),
		conf: row.confidence ?? 0,
		conflicts: 0,
		dups,
		impact: impactForConfidence(row.confidence)
	};
}

/**
 * TriageRow[] → KitTriageGroup[]: group by owner-scope label, ranked
 * strongest-first within/across groups (reusing `groupByScope`), each row mapped
 * to a `KitTriageCandidate`. Pure.
 */
export function toKitTriageGroups(rows: TriageRow[], now: Date = new Date()): KitTriageGroup[] {
	return groupByScope(rows).map((g) => ({
		scope: g.scope,
		items: g.rows.map((r) => toKitCandidate(r, now))
	}));
}

/**
 * TriageRow[] → KitApproval[]: only the high-impact candidates (the ones that
 * route to a second maintainer's signature). `first` is unknown from the list
 * row (the first approval lives on the decision, not the queue) so it reads
 * "pending"; a real first-approver lands with the decisions read. Pure.
 */
export function toKitApprovals(rows: TriageRow[], now: Date = new Date()): KitApproval[] {
	return rows
		.filter((r) => impactForConfidence(r.confidence) === 'high')
		.map((r) => ({
			id: r.signature,
			kanji: kindKanji(r.kind),
			title: r.title,
			scope: scopeLabel(r.owner_scope),
			first: 'pending',
			when: relativeAge(r.created_at, now),
			impact: 'high'
		}));
}

/**
 * A best-effort KitCandidateDetail for the focused candidate. The list route
 * carries no rich detail (learning · cause · evidence · conflict), so the pane
 * reflects what the row itself proves — the title as the learning, the scope as
 * the context, and the distribution scope chip. A full per-candidate detail read
 * is a follow-on. Returns a neutral honest-empty detail when no row is focused.
 */
export function toKitCandidateDetail(row: TriageRow | undefined): KitCandidateDetail {
	if (!row) {
		return {
			learning: 'Select a candidate to see its detail.',
			cause: '',
			context: '',
			evidence: [],
			conflict: { loser: '', winner: '' },
			scopes: []
		};
	}
	const scope = scopeLabel(row.owner_scope);
	return {
		learning: row.title,
		cause: '',
		context: scope === 'Unscoped' ? '' : scope,
		evidence: [],
		conflict: { loser: '', winner: '' },
		scopes: [scope]
	};
}
