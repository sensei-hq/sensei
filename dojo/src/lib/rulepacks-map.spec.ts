import { describe, it, expect } from 'vitest';
import { toKitRulePacks } from './rulepacks-map';
import type { LibraryPackWire } from './client-data';

const wire: LibraryPackWire[] = [
	{ slug: 'security-baseline', kanji: '盾', name: 'Security baseline', by: 'OWASP', note: 'app-sec', rules: ['no secrets', 'no PII logs'] },
	{ slug: 'tdd', kanji: null, name: 'TDD', by: 'XP', note: 'test-first', rules: ['red first'] }
];

describe('toKitRulePacks — library wire → KitRulePack', () => {
	it('maps slug→id, source→by, summary→note, rules passthrough, kanji kept', () => {
		const [a] = toKitRulePacks(wire);
		expect(a.id).toBe('security-baseline');
		expect(a.by).toBe('OWASP');
		expect(a.note).toBe('app-sec');
		expect(a.rules).toEqual(['no secrets', 'no PII logs']);
		expect(a.kanji).toBe('盾');
	});

	it('falls back to a default kanji when the pack has none', () => {
		expect(toKitRulePacks(wire)[1].kanji).toBe('守');
	});

	it('marks adopted only for slugs in the adopted set (default = none adopted)', () => {
		expect(toKitRulePacks(wire).every((p) => !p.adopted)).toBe(true);
		const [a, b] = toKitRulePacks(wire, new Set(['security-baseline']));
		expect(a.adopted).toBe(true);
		expect(b.adopted).toBe(false);
	});
});
