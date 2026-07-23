import { describe, expect, it } from 'vitest';
import { signalToneClass, alertToneClass, barMax, barPct } from './dojo2-health-view';
import type { KitHealthWeek } from './components/kit/types';

// The admin health helpers — the signal + alert tone maps and the
// contributions-vs-approvals bar geometry. Pure functions, no DOM.

describe('signalToneClass', () => {
	it('maps the known tone keys to token classes', () => {
		expect(signalToneClass('accent')).toBe('text-accent');
		expect(signalToneClass('success')).toBe('text-success');
		expect(signalToneClass('warning')).toBe('text-warning');
		expect(signalToneClass('ink')).toBe('text-ink');
	});

	it('falls back to neutral ink for an unknown key', () => {
		expect(signalToneClass('mystery')).toBe('text-ink');
	});
});

describe('alertToneClass', () => {
	it('tints a warning alert with the warning token', () => {
		expect(alertToneClass('warning')).toBe('text-warning');
	});

	it('tints anything else (resolved) with success', () => {
		expect(alertToneClass('resolved')).toBe('text-success');
	});
});

describe('bar geometry', () => {
	const weeks: KitHealthWeek[] = [
		{ wk: 'W1', c: 18, a: 12 },
		{ wk: 'W2', c: 22, a: 15 },
		{ wk: 'W3', c: 19, a: 17 },
		{ wk: 'W4', c: 26, a: 20 }
	];

	it('barMax is the tallest value across both series', () => {
		// the biggest of every c and a is W4's contributions = 26.
		expect(barMax(weeks)).toBe(26);
	});

	it('barMax is 0 for an empty series', () => {
		expect(barMax([])).toBe(0);
	});

	it('barPct scales a value against the shared max as a whole percent', () => {
		expect(barPct(26, 26)).toBe(100);
		expect(barPct(13, 26)).toBe(50);
		expect(barPct(12, 26)).toBe(46);
	});

	it('barPct clamps to 0 when the scale is 0 (no divide-by-zero)', () => {
		expect(barPct(5, 0)).toBe(0);
	});
});
