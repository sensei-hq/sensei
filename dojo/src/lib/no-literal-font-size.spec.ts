import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

// #132 (dojo/CLAUDE.md, frontend-svelte-guidelines §1): text sizes come from the
// 8-stop scale (`text-xs` … `text-4xl`), never a literal `font-size`. Computed
// sizes are allowed — a size derived from a prop (`{…}` / `${…}`) is not a
// literal — so the Avatar initial and a sized KanjiToken stay.
function svelteFiles(dir: string): string[] {
	return readdirSync(dir).flatMap((name) => {
		const path = join(dir, name);
		if (statSync(path).isDirectory()) return svelteFiles(path);
		return path.endsWith('.svelte') ? [path] : [];
	});
}

describe('dōjō text sizes', () => {
	it('no component sets a literal font-size', () => {
		const literal = /font-size:\s*[\d.]+(px|rem|em)/;
		// kavach's sign-in field label: third-party markup no utility class can
		// reach, and 0.6875rem is exactly the 11px `text-xs` stop.
		const allowed = ['src/lib/components/DojoSignIn.svelte: font-size: 0.6875rem;'];
		const offenders = svelteFiles('src').flatMap((file) =>
			readFileSync(file, 'utf8')
				.split('\n')
				.map((line, i) => [file, i + 1, line.trim()] as const)
				.filter(([, , line]) => literal.test(line))
				.filter(([f, , line]) => !allowed.includes(`${f}: ${line}`)),
		);
		expect(offenders).toEqual([]);
	});
});
