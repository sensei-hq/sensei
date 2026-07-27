import { describe, expect, it } from 'vitest';
import { createRulePacks } from './rulepacks-state.svelte';
import { rulePacks } from './components/kit/fixtures';

// The reactive adopt-toggle state for ScrRulePacks. Delegates the split to the
// pure `personal-view` module; here we prove toggling moves a pack between
// adopted and available and never mutates the seed. Runs under the .svelte.ts
// test transform (like dojo2-preview-state.spec.svelte.ts).

// The fixture split — derived so the credible-pack content can grow without
// re-hardcoding magic tallies in every assertion.
const ADOPTED = rulePacks.filter((p) => p.adopted).length;
const AVAILABLE = rulePacks.length - ADOPTED;

describe('createRulePacks — reactive adopt-toggle state', () => {
	it('seeds the split off the fixtures (adopted vs available)', () => {
		const s = createRulePacks(rulePacks);
		expect(s.adopted.length).toBe(ADOPTED);
		expect(s.available.length).toBe(AVAILABLE);
	});

	it('adopting an available pack moves it to the adopted set', () => {
		const s = createRulePacks(rulePacks);
		const target = rulePacks.find((p) => !p.adopted)!;
		expect(s.isAdopted(target.id)).toBe(false);
		s.toggle(target.id);
		expect(s.isAdopted(target.id)).toBe(true);
		expect(s.adopted.length).toBe(ADOPTED + 1);
		expect(s.available.length).toBe(AVAILABLE - 1);
	});

	it('dropping an adopted pack moves it to the available set', () => {
		const s = createRulePacks(rulePacks);
		const target = rulePacks.find((p) => p.adopted)!;
		s.toggle(target.id);
		expect(s.isAdopted(target.id)).toBe(false);
		expect(s.adopted.length).toBe(ADOPTED - 1);
		expect(s.available.length).toBe(AVAILABLE + 1);
	});

	it('never mutates the seed fixtures', () => {
		const before = rulePacks.map((p) => p.adopted);
		const s = createRulePacks(rulePacks);
		s.toggle(rulePacks[0].id);
		expect(rulePacks.map((p) => p.adopted)).toEqual(before);
	});
});
