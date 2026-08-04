import { describe, it, expect } from 'vitest';
import { shapeLibraryPacks } from './rulepacks-data';

describe('shapeLibraryPacks — view rows → library wire', () => {
	it('maps view fields (source→by, summary→note) and passes rules through', () => {
		const [p] = shapeLibraryPacks([
			{ slug: 'x', kanji: '技', name: 'X', source: 'src', summary: 's', rules: ['first', 'second'] }
		]);
		expect(p.rules).toEqual(['first', 'second']); // view already ordered by ordinal
		expect(p.by).toBe('src');
		expect(p.note).toBe('s');
		expect(p.kanji).toBe('技');
	});

	it('handles null summary / kanji / rules as honest empties', () => {
		const [p] = shapeLibraryPacks([
			{ slug: 'y', kanji: null, name: 'Y', source: 'src', summary: null, rules: null }
		]);
		expect(p.note).toBe('');
		expect(p.kanji).toBeNull();
		expect(p.rules).toEqual([]);
	});
});
