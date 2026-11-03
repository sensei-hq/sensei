// Every error, refusal and wait in the dōjō renders through the shared Notice
// (@sensei/ui; docs/architecture/notice.md, docs/plan/notice-adoption.md P4).
// This spec keeps it that way: a NEW red line, red box or `role="alert"` written
// by hand fails here, with a pointer to Notice — the same guard the desktop app
// has (app/src/lib/error-surfaces.spec.ts).
//
// What stays is listed below, each with why. None is an error message.
import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

const DANGER = /text-danger|bg-danger|border-danger|role="alert"/g;

/** file (relative to src/) → how many danger-coloured uses it may keep, and why. */
const ALLOWED: Record<string, { count: number; why: string }> = {
	'lib/components/kit/Btn.svelte': { count: 2, why: 'the danger button variant (fill and edge)' },
	'lib/components/screens/ViewersList.svelte': { count: 1, why: 'the remove link turns danger on hover' },
	'lib/components/screens/TeamsList.svelte': { count: 1, why: 'the remove link turns danger on hover' },
	'lib/components/screens/ScrContributions.svelte': { count: 3, why: "the 'declined' contribution chip tone" }
};

function svelteFiles(dir: string): string[] {
	return readdirSync(dir).flatMap((name) => {
		const p = join(dir, name);
		if (statSync(p).isDirectory()) return svelteFiles(p);
		return p.endsWith('.svelte') && !p.endsWith('.harness.svelte') ? [p] : [];
	});
}

describe('error surfaces render through Notice', () => {
	const found = new Map<string, number>();
	for (const file of svelteFiles('src')) {
		const n = readFileSync(file, 'utf8').match(DANGER)?.length ?? 0;
		if (n > 0) found.set(file.replace(/^src\//, ''), n);
	}

	it('no file colours an error by hand — say it with <Notice> (@sensei/ui)', () => {
		const extra = [...found].filter(([f, n]) => n > (ALLOWED[f]?.count ?? 0)).map(([f, n]) => `${f} (${n})`);
		expect(extra, 'hand-written error colour or role="alert"; use Notice instead').toEqual([]);
	});

	it('the allowlist has no stale entries', () => {
		const stale = Object.entries(ALLOWED).filter(([f, a]) => (found.get(f) ?? 0) < a.count).map(([f]) => f);
		expect(stale, 'shrink or drop these entries').toEqual([]);
	});
});
