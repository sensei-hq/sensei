import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

// #106: no screen a person uses shows fabricated data. A view that imports the
// kit's mockup fixtures renders an invented org, person, ladder or invoice as if
// it were theirs. Fixtures stay for specs and component harnesses only.
//
// Every view now reads its `/v1` data or states what is absent; this keeps it so.
// Fixture modules are `kit/fixtures`, `*.fixtures` and `*.mock`.
function sources(dir: string): string[] {
	return readdirSync(dir).flatMap((name) => {
		const p = join(dir, name);
		if (statSync(p).isDirectory()) return sources(p);
		const isTest = /\.(spec|test)\./.test(p) || p.includes('.harness.');
		return /\.(ts|svelte)$/.test(p) && !isTest ? [p] : [];
	});
}

describe('fixtures in views (#106)', () => {
	it('no view imports a fixture or mock module', () => {
		const importers = sources('src')
			.filter((f) => !/(kit\/fixtures|\.fixtures|\.mock)\.ts$/.test(f))
			.filter((f) => /from\s+['"][^'"]*(kit\/fixtures|\.fixtures|\.mock)['"]/.test(readFileSync(f, 'utf8')))
			.map((f) => f.replace(/^src\//, ''))
			.sort();
		expect(importers).toEqual([]);
	});
});
