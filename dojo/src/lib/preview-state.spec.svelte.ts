import { describe, expect, it } from 'vitest';
import { createPreviewStore } from '$lib/preview-state.svelte';
import { PV_PROJECTS } from '$lib/preview-data';

// The rune-store spec (`.spec.svelte.ts` so $state compiles). Proves the store
// holds the selected project + classification override and delegates the
// resolution to preview-view: selecting a project recomputes the effective
// constitution, and a company↔client override changes which rungs apply.

describe('createPreviewStore — selection', () => {
	it('starts on the given initial project', () => {
		const s = createPreviewStore('globex');
		expect(s.project.id).toBe('globex');
		expect(s.effectiveKind).toBe('client');
	});

	it('falls back to the first project for an unknown id', () => {
		const s = createPreviewStore('nope');
		expect(s.project.id).toBe(PV_PROJECTS[0].id);
	});

	it('selects a different project and recomputes the ladder', () => {
		const s = createPreviewStore('globex');
		s.select('site');
		expect(s.project.id).toBe('site');
		// personal project → no company/client rung
		const scopes = s.constitution.ladder.map((r) => r.scope);
		expect(scopes).not.toContain('company');
		expect(scopes).not.toContain('client');
	});
});

describe('createPreviewStore — resolved constitution', () => {
	it('exposes the composed constitution + counts for the current project', () => {
		const s = createPreviewStore('auth'); // company
		expect(s.constitution.totalRules).toBeGreaterThan(0);
		expect(s.constitution.lockedCount).toBeGreaterThan(0);
		expect(s.constitution.scopeCount).toBe(s.constitution.ladder.length);
		// company project locks coverage against the project relax
		expect(s.constitution.conflicts.find((c) => c.topic === 'coverage')?.locked).toBe(true);
	});
});

describe('createPreviewStore — classification override', () => {
	it('reclassifying a company project as client switches the Client rung on', () => {
		const s = createPreviewStore('auth'); // company
		expect(s.constitution.ladder.some((r) => r.scope === 'client')).toBe(false);
		expect(s.isOverridden).toBe(false);
		s.reclassify('client');
		expect(s.effectiveKind).toBe('client');
		expect(s.isOverridden).toBe(true);
		expect(s.constitution.ladder.some((r) => r.scope === 'client')).toBe(true);
	});

	it('reclassifying a client project as company switches the Client rung off', () => {
		const s = createPreviewStore('globex'); // client
		expect(s.constitution.ladder.some((r) => r.scope === 'client')).toBe(true);
		s.reclassify('company');
		expect(s.effectiveKind).toBe('company');
		expect(s.constitution.ladder.some((r) => r.scope === 'client')).toBe(false);
	});

	it('reset clears the override back to the base classification', () => {
		const s = createPreviewStore('auth');
		s.reclassify('client');
		expect(s.isOverridden).toBe(true);
		s.resetClassification();
		expect(s.isOverridden).toBe(false);
		expect(s.effectiveKind).toBe('company');
	});

	it('an override is per-project — switching projects drops it', () => {
		const s = createPreviewStore('auth');
		s.reclassify('client');
		expect(s.isOverridden).toBe(true);
		s.select('globex');
		expect(s.isOverridden).toBe(false);
		expect(s.effectiveKind).toBe('client'); // globex's base kind
	});

	it('keeps the coverage lock through a reclassification (lock is scope-independent)', () => {
		const s = createPreviewStore('auth');
		s.reclassify('client');
		expect(s.constitution.conflicts.find((c) => c.topic === 'coverage')?.locked).toBe(true);
	});
});
