import { describe, expect, it } from 'vitest';
import { classTone, phaseTone, roleTone, kindTone, iconClass, ICON_FALLBACK } from './index';
import { iconClass as iconFromName } from './Icon.svelte';

// The vocab maps decide every tone in one place; the lookups default safely and
// the icon-name→i-solar bridge resolves a static, safelisted UnoCSS class.
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

describe('Icon Solar mapping', () => {
	it('maps a logical name to its static i-solar:*-linear class', () => {
		expect(iconFromName('folder')).toBe('i-solar:folder-linear');
		expect(iconFromName('eye')).toBe('i-solar:eye-linear');
		expect(iconFromName('scale')).toBe('i-solar:scale-linear');
		expect(iconFromName('command')).toBe('i-solar:command-linear');
	});

	it('falls back to a neutral icon for an unmapped name', () => {
		expect(iconFromName('no-such-icon')).toBe(ICON_FALLBACK);
	});

	it('re-exports iconClass from the barrel', () => {
		expect(iconClass).toBe(iconFromName);
	});
});
