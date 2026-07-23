import { describe, expect, it } from 'vitest';
import { classTone, phaseTone, roleTone, kindTone } from './index';

// The vocab maps decide every tone in one place; the lookups default safely.
describe('kit vocab lookups', () => {
	it('classTone resolves a known kind and defaults to company', () => {
		expect(classTone('client').label).toBe('client');
		expect(classTone('client').text).toBe('text-accent');
		expect(classTone(undefined).label).toBe('company');
		expect(classTone('nope').text).toBe('text-ink-soft');
	});

	it('phaseTone lights the right number of steps and defaults to watch', () => {
		expect(phaseTone('watch').step).toBe(1);
		expect(phaseTone('notice').step).toBe(2);
		expect(phaseTone('adopt').step).toBe(3);
		expect(phaseTone(null).label).toBe('watch');
	});

	it('roleTone returns a tone for a known role and undefined otherwise', () => {
		expect(roleTone('maintainer')?.label).toBe('maintainer');
		expect(roleTone('maintainer')?.icon).toBe('settings');
		expect(roleTone('ghost')).toBeUndefined();
		expect(roleTone(null)).toBeUndefined();
	});

	it('kindTone resolves org kinds and defaults to employer', () => {
		expect(kindTone('client').text).toBe('text-accent');
		expect(kindTone('community').text).toBe('text-success');
		expect(kindTone(undefined).kanji).toBe('社');
	});
});
