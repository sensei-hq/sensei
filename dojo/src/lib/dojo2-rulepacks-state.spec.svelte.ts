import { describe, expect, it } from 'vitest';
import { createRulePacks } from './dojo2-rulepacks-state.svelte';
import { rulePacks } from './components/kit/fixtures';

// The reactive adopt-toggle state for ScrRulePacks. Delegates the split to the
// pure `dojo2-personal-view` module; here we prove toggling moves a pack between
// adopted and available and never mutates the seed. Runs under the .svelte.ts
// test transform (like dojo2-preview-state.spec.svelte.ts).

describe('createRulePacks — reactive adopt-toggle state', () => {
	it('seeds the split off the fixtures (3 adopted, 2 available)', () => {
		const s = createRulePacks(rulePacks);
		expect(s.adopted.length).toBe(3);
		expect(s.available.length).toBe(2);
	});

	it('adopting an available pack moves it to the adopted set', () => {
		const s = createRulePacks(rulePacks);
		const target = rulePacks.find((p) => !p.adopted)!;
		expect(s.isAdopted(target.id)).toBe(false);
		s.toggle(target.id);
		expect(s.isAdopted(target.id)).toBe(true);
		expect(s.adopted.length).toBe(4);
		expect(s.available.length).toBe(1);
	});

	it('dropping an adopted pack moves it to the available set', () => {
		const s = createRulePacks(rulePacks);
		const target = rulePacks.find((p) => p.adopted)!;
		s.toggle(target.id);
		expect(s.isAdopted(target.id)).toBe(false);
		expect(s.adopted.length).toBe(2);
		expect(s.available.length).toBe(3);
	});

	it('never mutates the seed fixtures', () => {
		const before = rulePacks.map((p) => p.adopted);
		const s = createRulePacks(rulePacks);
		s.toggle(rulePacks[0].id);
		expect(rulePacks.map((p) => p.adopted)).toEqual(before);
	});
});
