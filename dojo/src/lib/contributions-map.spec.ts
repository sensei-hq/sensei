import { describe, it, expect } from 'vitest';
import {
	kindKanji,
	artifactStatus,
	isAdopted,
	scopeLabel,
	toKitContribution,
	toKitDownstream,
	type ContributionRow,
	type DownstreamRow
} from './contributions-map';

// Pure daemon→kit mappers for the Contributions screen (F5). No DOM.

const NOW = new Date('2026-08-03T12:00:00Z');
const HOURS_AGO_2 = new Date('2026-08-03T10:00:00Z').toISOString();

describe('kindKanji — artifact_kind → brand glyph', () => {
	it('maps each known kind', () => {
		expect(kindKanji('principle')).toBe('理');
		expect(kindKanji('pattern')).toBe('紋');
		expect(kindKanji('guard')).toBe('守');
		expect(kindKanji('skill')).toBe('技');
	});
	it('falls back to the neutral share glyph on unknown (never a wrong intent)', () => {
		expect(kindKanji('mystery')).toBe('共');
	});
});

describe('artifactStatus — dojo.artifact_status → screen status', () => {
	it('maps submitted→pending, published→approved, archived→declined', () => {
		expect(artifactStatus('submitted')).toBe('pending');
		expect(artifactStatus('published')).toBe('approved');
		expect(artifactStatus('archived')).toBe('declined');
	});
	it('defaults an unknown status to pending (in triage), never approved', () => {
		expect(artifactStatus('weird')).toBe('pending');
	});
});

describe('isAdopted — inbox_state → adopted', () => {
	it('is true only for applied or pinned', () => {
		expect(isAdopted('applied')).toBe(true);
		expect(isAdopted('pinned')).toBe(true);
		expect(isAdopted('pending')).toBe(false);
		expect(isAdopted('muted')).toBe(false);
	});
});

describe('scopeLabel — honest scope, never fabricated', () => {
	it('joins level + name when present', () => {
		expect(scopeLabel({ level: 'Stack', name: 'Rust' })).toBe('Stack · Rust');
	});
	it('is empty for an empty/absent scope', () => {
		expect(scopeLabel({})).toBe('');
		expect(scopeLabel(null)).toBe('');
	});
});

describe('toKitContribution — a shared artifact', () => {
	const row: ContributionRow = {
		kind: 'pattern',
		title: 'prefer Result over throw',
		status: 'published',
		attribution: { mode: 'anonymous' },
		scope: { level: 'Stack', name: 'Rust' },
		created_at: HOURS_AGO_2,
		dest: 'Acme'
	};
	it('maps kind/title/dest/scope/status/when + the anonymity marker from attribution', () => {
		const c = toKitContribution(row, NOW);
		expect(c).toMatchObject({
			kanji: '紋',
			title: 'prefer Result over throw',
			dest: 'Acme',
			scope: 'Stack · Rust',
			status: 'approved',
			anonymous: true
		});
		expect(c.when).toBeTruthy();
	});
	it('is named (anonymous false) when attribution.mode is not anonymous or absent', () => {
		expect(toKitContribution({ ...row, attribution: { mode: 'named' } }, NOW).anonymous).toBe(false);
		expect(toKitContribution({ ...row, attribution: null }, NOW).anonymous).toBe(false);
	});
	it('falls back dest to — when the tenant name is missing (honest, not blank)', () => {
		expect(toKitContribution({ ...row, dest: null }, NOW).dest).toBe('—');
	});
});

describe('toKitDownstream — an approved-for-you item', () => {
	const row: DownstreamRow = {
		id: 'art-1',
		state: 'pinned',
		created_at: HOURS_AGO_2,
		kind: 'guard',
		title: 'no secrets in logs',
		scope: { level: 'Company', name: 'Acme' },
		from: 'Acme'
	};
	it('maps id/kind/title/from/scope + adopted from the inbox state', () => {
		const d = toKitDownstream(row, NOW);
		expect(d).toMatchObject({ id: 'art-1', kanji: '守', title: 'no secrets in logs', from: 'Acme', adopted: true, kind: 'guard' });
	});
	it('is unadopted (pinnable) when pending', () => {
		expect(toKitDownstream({ ...row, state: 'pending' }, NOW).adopted).toBe(false);
	});
});
