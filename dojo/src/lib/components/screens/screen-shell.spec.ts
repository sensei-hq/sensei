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

// Screens that are not Scr components: the relay inbox's two panes and the
// routes that render their own screen rather than a Scr one.
const OTHER_SCREENS = [
	'src/lib/relay/RelayList.svelte',
	'src/lib/relay/RelayDetail.svelte',
	'src/routes/(app)/you/(inbox)/+page.svelte',
	'src/routes/(app)/you/(inbox)/runs/[run_id]/+page.svelte',
	'src/routes/(app)/you/invites/accept/+page.svelte'
];

const NOT_YET = new Set<string>([]);

describe('dōjō screens use the shared Screen layout (#293)', () => {
	it('every screen not listed as pending uses Screen', () => {
		expect(screens.filter((f) => !NOT_YET.has(f) && !usesScreen(f))).toEqual([]);
	});

	it('a screen that has moved to Screen leaves the pending list', () => {
		expect([...NOT_YET].filter(usesScreen)).toEqual([]);
	});

	it('the relay panes and the routes with their own screen use Screen too', () => {
		expect(OTHER_SCREENS.filter((f) => !/<Screen[\s>]/.test(readFileSync(f, 'utf8')))).toEqual([]);
	});

	it('the AppShell main column does not scroll — each screen body is the one scroller', () => {
		let shell = readFileSync('src/lib/components/kit/AppShell.svelte', 'utf8');
		// The markup outside its <script> blocks: every block, any case, until none is left.
		for (let prev = ''; prev !== shell; ) {
			prev = shell;
			shell = shell.replace(/<script\b[\s\S]*?<\/script[^>]*>/gi, '');
		}
		const column = shell.match(/<div[^>]*data-component="app-main"[^>]*>/)?.[0];
		expect(column).toBeDefined();
		expect(column).not.toMatch(/overflow-(auto|scroll|y-auto|y-scroll)\b/);
	});

	it('the pending list names only screens that exist', () => {
		expect([...NOT_YET].filter((f) => !screens.includes(f))).toEqual([]);
	});
});
