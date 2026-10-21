import { describe, expect, it } from 'vitest';
import { createWorkspacePreview } from './state.svelte';
import { ladder, conflicts } from '../components/kit/fixtures';
import type { KitWorkspace } from '../components/kit/types';

// The reactive drill-in state for ScrWorkspacePreview — the selected rung
// (`active`) and the by-layer / consolidated `view` toggle. Delegates all
// resolution to the pure `preview/view` module; here we only prove the
// UI state reacts. Runs under the .svelte.ts test transform (like
// preview-state.spec.svelte.ts).

const company: KitWorkspace = {
	id: 'c',
	name: 'lumen-auth',
	repo: 'acme/lumen-auth',
	classification: 'company',
	phase: 'notice'
};

describe('createWorkspacePreview — reactive drill-in state', () => {
	it('opens on the workspace rung in the by-layer view', () => {
		const p = createWorkspacePreview(company, ladder, conflicts);
		expect(p.active).toBe('project');
		expect(p.view).toBe('layer');
	});

	it('exposes the resolved rungs, effective rules, discards and counts', () => {
		const p = createWorkspacePreview(company, ladder, conflicts);
		// every composed scope (company · client · personal · stack) + the synth workspace anchor
		expect(p.rungs.length).toBe(5);
		expect(p.effective.length).toBeGreaterThan(0);
		expect(p.discarded.length).toBe(conflicts.length);
		expect(p.locks).toBe(4); // company (3) + client (1)
		expect(p.showConflicts).toBe(true);
	});

	it('setActive focuses a rung', () => {
		const p = createWorkspacePreview(company, ladder, conflicts);
		p.setActive('company');
		expect(p.active).toBe('company');
	});

	it('setView switches between layer and consolidated', () => {
		const p = createWorkspacePreview(company, ladder, conflicts);
		p.setView('consolidated');
		expect(p.view).toBe('consolidated');
		p.setView('layer');
		expect(p.view).toBe('layer');
	});

	it('jumpTo focuses the rung owning a level and returns to the layer view', () => {
		const p = createWorkspacePreview(company, ladder, conflicts);
		p.setView('consolidated');
		// jump to a Company-scoped rule → focuses the company rung, back to layers.
		p.jumpTo('Company');
		expect(p.active).toBe('company');
		expect(p.view).toBe('layer');
	});

	it('a personal workspace hides conflicts and discards nothing', () => {
		const personal: KitWorkspace = { ...company, classification: 'personal' };
		const p = createWorkspacePreview(personal, ladder, conflicts);
		expect(p.showConflicts).toBe(false);
		expect(p.discarded).toEqual([]);
	});
});
