import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

// #106: no screen a person uses shows fabricated data. A view that imports the
// kit's mockup fixtures renders an invented org, person, ladder or invoice as if
// it were theirs. Fixtures stay for specs and component harnesses only.
//
// A ratchet: the loaders below still read fixtures and are being moved to their
// `/v1` reads or to a stated absence, one per change. The list only shrinks, and
// a new importer fails at once.
const STILL_ON_FIXTURES = [
	'routes/(app)/org/[slug]/[section]/+page.ts',
	'routes/(app)/org/[slug]/workspaces/[id]/+page.ts'
];

function sources(dir: string): string[] {
	return readdirSync(dir).flatMap((name) => {
		const p = join(dir, name);
		if (statSync(p).isDirectory()) return sources(p);
		const isTest = /\.(spec|test)\./.test(p) || p.includes('.harness.');
		return /\.(ts|svelte)$/.test(p) && !isTest ? [p] : [];
	});
}

describe('fixtures in views (#106)', () => {
	it('only the loaders still being moved import the kit fixtures', () => {
		const importers = sources('src')
			.filter((f) => !f.endsWith('kit/fixtures.ts'))
			.filter((f) => /from\s+['"][^'"]*kit\/fixtures['"]/.test(readFileSync(f, 'utf8')))
			.map((f) => f.replace(/^src\//, ''))
			.sort();
		expect(importers).toEqual(STILL_ON_FIXTURES);
	});
});
