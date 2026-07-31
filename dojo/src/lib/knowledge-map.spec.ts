// Unit tests for the Knowledge wire→kit mapper (`knowledge-map.ts`): the scope
// label derivation and the KnowledgeLibraryWire → KitKnowledge projection.
import { describe, expect, it } from 'vitest';
import { toKitKnowledge, scopeLabel } from './knowledge-map';
import type { KnowledgeArtifactWire, KnowledgeLibraryWire } from './client-data';

const NOW = new Date('2026-07-30T00:00:00Z');

function art(over: Partial<KnowledgeArtifactWire> = {}): KnowledgeArtifactWire {
	return {
		id: 'a1',
		kind: 'principle',
		title: 'Idempotency key on money mutations',
		scope: { team: 'Payments' },
		adopted_count: 3,
		created_at: '2026-04-30T00:00:00Z',
		...over
	};
}

describe('scopeLabel', () => {
	it('formats team / project / stack with a middot', () => {
		expect(scopeLabel({ team: 'Payments' })).toBe('Team · Payments');
		expect(scopeLabel({ project: 'ledger' })).toBe('Project · ledger');
		expect(scopeLabel({ stack: 'React' })).toBe('Stack · React');
	});
	it('reads company as a bare label (or with a value)', () => {
		expect(scopeLabel({ company: true })).toBe('Company');
		expect(scopeLabel({ company: 'Acme' })).toBe('Company · Acme');
	});
	it('falls back to Unscoped for empty / non-object scope', () => {
		expect(scopeLabel(null)).toBe('Unscoped');
		expect(scopeLabel({})).toBe('Unscoped');
		expect(scopeLabel('nope')).toBe('Unscoped');
	});
});

describe('toKitKnowledge', () => {
	it('maps the prune policy label from retention_days', () => {
		expect(toKitKnowledge({ retention_days: 90, active: [], pending: [], catalog: [] }).prunePolicy).toBe(
			'Prune after 90 days unused'
		);
		expect(toKitKnowledge({ retention_days: null, active: [], pending: [], catalog: [] }).prunePolicy).toBe(
			'No prune policy'
		);
	});
	it('maps an active row: kind kanji, scope label, adoption reach, published age', () => {
		const lib: KnowledgeLibraryWire = { retention_days: 90, active: [art()], pending: [], catalog: [] };
		const k = toKitKnowledge(lib, NOW);
		expect(k.active[0]).toMatchObject({
			kanji: '則', // principle
			title: 'Idempotency key on money mutations',
			scope: 'Team · Payments',
			adopted: '3 repos'
		});
		expect(k.active[0].age).toMatch(/^published /);
	});
	it('singularises a single adopting repo', () => {
		const lib: KnowledgeLibraryWire = { retention_days: null, active: [art({ adopted_count: 1 })], pending: [], catalog: [] };
		expect(toKitKnowledge(lib, NOW).active[0].adopted).toBe('1 repo');
	});
	it('maps a pending row with an "unused" age and no adoption reach', () => {
		const lib: KnowledgeLibraryWire = {
			retention_days: 90,
			active: [],
			pending: [art({ kind: 'guard', adopted_count: 0 })],
			catalog: []
		};
		const row = toKitKnowledge(lib, NOW).pending[0];
		expect(row.kanji).toBe('守'); // guard
		expect(row.adopted).toBeUndefined();
		expect(row.age).toMatch(/^unused /);
	});
	it('maps a catalog item: kind chip + glyph + scope', () => {
		const lib: KnowledgeLibraryWire = {
			retention_days: null,
			active: [],
			pending: [],
			catalog: [art({ kind: 'skill', title: 'auth-boundary reviewer', scope: { company: true } })]
		};
		expect(toKitKnowledge(lib, NOW).catalog[0]).toEqual({
			kanji: '技',
			title: 'auth-boundary reviewer',
			kind: 'skill',
			scope: 'Company'
		});
	});
});
