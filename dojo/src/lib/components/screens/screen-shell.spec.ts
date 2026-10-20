import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

// #293: every dōjō screen lays out through the shared `Screen` (@sensei/ui) —
// its SectionHead stays in place over one scroll body — instead of scrolling
// with the AppShell's main column. NOT_YET lists the screens still to move; it
// only shrinks.
const DIR = 'src/lib/components/screens';
const screens = readdirSync(DIR).filter((f) => /^Scr\w+\.svelte$/.test(f)).sort();
const usesScreen = (f: string) => /<Screen[\s>]/.test(readFileSync(join(DIR, f), 'utf8'));

const NOT_YET = new Set<string>([]);

describe('dōjō screens use the shared Screen layout (#293)', () => {
	it('every screen not listed as pending uses Screen', () => {
		expect(screens.filter((f) => !NOT_YET.has(f) && !usesScreen(f))).toEqual([]);
	});

	it('a screen that has moved to Screen leaves the pending list', () => {
		expect([...NOT_YET].filter(usesScreen)).toEqual([]);
	});

	it('the pending list names only screens that exist', () => {
		expect([...NOT_YET].filter((f) => !screens.includes(f))).toEqual([]);
	});
});
