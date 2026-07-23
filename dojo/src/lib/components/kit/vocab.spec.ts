import { describe, expect, it } from 'vitest';
import { classTone, phaseTone, roleTone, kindTone, glyphClass } from './index';
import { glyphClass as glyphFromIcon } from './Icon.svelte';

// The vocab maps decide every tone in one place; the lookups default safely and
// the icon-name→i-glyph bridge resolves both aliased and pass-through names.
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

describe('Icon glyph mapping', () => {
	it('aliases the kit Solar names whose glyph key differs', () => {
		expect(glyphFromIcon('command')).toBe('i-glyph:terminal');
		expect(glyphFromIcon('lock-keyhole')).toBe('i-glyph:lock');
		expect(glyphFromIcon('code-2')).toBe('i-glyph:code');
	});

	it('passes an unaliased name straight through', () => {
		expect(glyphFromIcon('bell')).toBe('i-glyph:bell');
		expect(glyphFromIcon('folder')).toBe('i-glyph:folder');
	});

	it('re-exports glyphClass from the barrel', () => {
		expect(glyphClass).toBe(glyphFromIcon);
	});
});
