// The dojo2 org maintainer Triage/Approvals wire→kit mapper. Deterministic `now`.
import { describe, expect, it } from 'vitest';
import type { TriageRow } from './triage-data';
import {
	impactForConfidence,
	toKitCandidate,
	toKitTriageGroups,
	toKitApprovals,
	toKitCandidateDetail
} from './dojo2-triage-map';

const NOW = new Date('2026-07-23T12:00:00Z');

function row(over: Partial<TriageRow> = {}): TriageRow {
	return {
		signature: 'sig-1',
		artifact_id: 'a1',
		kind: 'pattern',
		title: 'Idempotency key on money-moving mutations',
		owner_scope: { label: 'Payments' },
		confidence: 0.8,
		contributor_count: 3,
		similarity: null,
		nearest_artifact_id: null,
		state: 'queued',
		created_at: '2026-07-22T12:00:00Z',
		...over
	};
}

describe('impactForConfidence', () => {
	it('reads high at ≥0.90, else normal', () => {
		expect(impactForConfidence(0.95)).toBe('high');
		expect(impactForConfidence(0.9)).toBe('high');
		expect(impactForConfidence(0.7)).toBe('normal');
		expect(impactForConfidence(null)).toBe('normal');
	});
});

describe('toKitCandidate', () => {
	it('maps the row onto the kit candidate', () => {
		const c = toKitCandidate(row(), NOW);
		expect(c.id).toBe('sig-1');
		expect(c.kanji).toBe('紋'); // pattern glyph
		expect(c.title).toBe('Idempotency key on money-moving mutations');
		expect(c.conf).toBe(0.8);
		expect(c.impact).toBe('normal');
		expect(c.conflicts).toBe(0);
		expect(c.origin).toBe('3 contributors · 1d');
	});
	it('singular contributor + flags a dup when similarity ≥0.75 with a nearest', () => {
		const c = toKitCandidate(row({ contributor_count: 1, similarity: 0.8, nearest_artifact_id: 'a2' }), NOW);
		expect(c.origin).toBe('1 contributor · 1d');
		expect(c.dups).toBe(1);
	});
	it('no dup below the flag band or without a nearest', () => {
		expect(toKitCandidate(row({ similarity: 0.5, nearest_artifact_id: 'a2' }), NOW).dups).toBe(0);
		expect(toKitCandidate(row({ similarity: 0.9, nearest_artifact_id: null }), NOW).dups).toBe(0);
	});
	it('defaults conf to 0 for a null confidence', () => {
		expect(toKitCandidate(row({ confidence: null }), NOW).conf).toBe(0);
	});
});

describe('toKitTriageGroups', () => {
	it('groups by scope and ranks strongest-first', () => {
		const groups = toKitTriageGroups(
			[
				row({ signature: 'p-lo', owner_scope: { label: 'Payments' }, confidence: 0.4 }),
				row({ signature: 'p-hi', owner_scope: { label: 'Payments' }, confidence: 0.95 }),
				row({ signature: 'auth', owner_scope: { label: 'Auth' }, confidence: 0.6 })
			],
			NOW
		);
		const payments = groups.find((g) => g.scope === 'Payments');
		expect(payments?.items.map((i) => i.id)).toEqual(['p-hi', 'p-lo']);
		// The strongest group (Payments, 0.95) sorts ahead of Auth (0.6).
		expect(groups[0].scope).toBe('Payments');
	});
	it('returns [] for no rows', () => {
		expect(toKitTriageGroups([], NOW)).toEqual([]);
	});
});

describe('toKitApprovals', () => {
	it('includes only high-impact (≥0.90) candidates, marked pending first-approval', () => {
		const approvals = toKitApprovals(
			[row({ signature: 'hi', confidence: 0.95 }), row({ signature: 'lo', confidence: 0.7 })],
			NOW
		);
		expect(approvals.map((a) => a.id)).toEqual(['hi']);
		expect(approvals[0].impact).toBe('high');
		expect(approvals[0].first).toBe('pending');
		expect(approvals[0].scope).toBe('Payments');
	});
});

describe('toKitCandidateDetail', () => {
	it('projects the focused row onto a best-effort detail', () => {
		const d = toKitCandidateDetail(row());
		expect(d.learning).toBe('Idempotency key on money-moving mutations');
		expect(d.context).toBe('Payments');
		expect(d.scopes).toEqual(['Payments']);
	});
	it('returns an honest-empty detail when no row is focused', () => {
		const d = toKitCandidateDetail(undefined);
		expect(d.learning).toMatch(/select a candidate/i);
		expect(d.scopes).toEqual([]);
		expect(d.evidence).toEqual([]);
	});
});
