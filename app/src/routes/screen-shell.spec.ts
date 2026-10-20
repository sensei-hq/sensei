import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

// #293: every observatory and project screen lays out through `Screen` — one
// header that stays in place over one scroll body — instead of improvising its
// own header and scroll. NOT_YET lists the screens still to move; it only shrinks.
const ROOT = 'src/routes';
const GROUPS = ['(observatory)', '(project)'];

function pages(dir: string): string[] {
	return readdirSync(dir).flatMap((name) => {
		const path = join(dir, name);
		if (statSync(path).isDirectory()) return pages(path);
		return name === '+page.svelte' ? [relative(ROOT, path)] : [];
	});
}

const NOT_YET = new Set([
		'(observatory)/+page.svelte',
		'(observatory)/activity-logs/+page.svelte',
		'(observatory)/consolidation/+page.svelte',
		'(observatory)/dojo/connections/+page.svelte',
		'(observatory)/dojo/sharing/+page.svelte',
		'(observatory)/help/+page.svelte',
		'(observatory)/impact/+page.svelte',
		'(observatory)/insights/+page.svelte',
		'(observatory)/instruments/+page.svelte',
		'(observatory)/knowledge-sources/+page.svelte',
		'(observatory)/learnings/+page.svelte',
		'(observatory)/libraries/+page.svelte',
		'(observatory)/projects/+page.svelte',
		'(observatory)/sessions/+page.svelte',
		'(observatory)/share-review/+page.svelte',
		'(observatory)/traceability/+page.svelte',
		'(observatory)/upgrades/+page.svelte',
]);

const source = (page: string) => readFileSync(join(ROOT, page), 'utf8');
// A page with no markup only redirects (its `+page.ts` throws); it is not a screen.
const hasMarkup = (page: string) => source(page).replace(/<script[\s\S]*?<\/script>/, '').trim() !== '';
const all = GROUPS.flatMap((g) => pages(join(ROOT, g))).filter(hasMarkup).sort();
const usesScreen = (page: string) => /<Screen[\s>]/.test(source(page));

describe('screens use the shared Screen layout (#293)', () => {
	it('every screen not listed as pending uses Screen', () => {
		expect(all.filter((p) => !NOT_YET.has(p) && !usesScreen(p))).toEqual([]);
	});

	it('a screen that has moved to Screen leaves the pending list', () => {
		expect([...NOT_YET].filter((p) => usesScreen(p))).toEqual([]);
	});

	it('the pending list names only screens that exist', () => {
		expect([...NOT_YET].filter((p) => !all.includes(p))).toEqual([]);
	});
});
