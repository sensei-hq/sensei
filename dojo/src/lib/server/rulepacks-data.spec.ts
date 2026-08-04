import { describe, it, expect } from 'vitest';
import { shapeLibraryPacks } from './rulepacks-data';

describe('shapeLibraryPacks — joined rows → library wire', () => {
	it('flattens embedded rules to statements in ordinal order', () => {
		const [p] = shapeLibraryPacks([
			{
				slug: 'x',
				kanji: '技',
				name: 'X',
				source: 'src',
				summary: 's',
				rule_pack_rules: [
					{ statement: 'second', ordinal: 2 },
					{ statement: 'first', ordinal: 1 }
				]
			}
		]);
		expect(p.rules).toEqual(['first', 'second']);
		expect(p.by).toBe('src');
		expect(p.note).toBe('s');
		expect(p.kanji).toBe('技');
	});

	it('handles null summary / kanji / rules as honest empties', () => {
		const [p] = shapeLibraryPacks([
			{ slug: 'y', kanji: null, name: 'Y', source: 'src', summary: null, rule_pack_rules: null }
		]);
		expect(p.note).toBe('');
		expect(p.kanji).toBeNull();
		expect(p.rules).toEqual([]);
	});
});
