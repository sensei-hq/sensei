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
// A hand-built warning box — a warning tint on a bordered box in one class string (L5, r1).
const WARNING_BOX = /class="[^"]*(?:bg-warning-soft[^"]*\bborder\b|\bborder\b[^"]*bg-warning-soft)[^"]*"/g;

/** file (relative to src/) → how many danger-coloured uses it may keep, and why. */
const ALLOWED: Record<string, { count: number; why: string }> = {
	'lib/components/kit/Btn.svelte': { count: 2, why: 'the danger button variant (fill and edge)' },
	'lib/components/screens/ViewersList.svelte': { count: 1, why: 'the remove link turns danger on hover' },
	'lib/components/screens/TeamsList.svelte': { count: 1, why: 'the remove link turns danger on hover' },
	'lib/components/screens/ScrContributions.svelte': { count: 3, why: "the 'declined' contribution chip tone" },
	'lib/components/screens/ScrTriage.svelte': { count: 1, why: "a candidate's conflict, shown as content beside the rule it loses to — not a notice" },
	'lib/components/RelayOfflineBanner.svelte': { count: 2, why: 'no route imports it (dead code for L6); remove rather than migrate' }
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
		const src = readFileSync(file, 'utf8');
		const n = (src.match(DANGER)?.length ?? 0) + (src.match(WARNING_BOX)?.length ?? 0);
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
