import { describe, it, expect } from 'vitest';
import { getInitials } from './initials';

describe('getInitials', () => {
	it('takes the first letter of the first two words', () => {
		expect(getInitials('Jerry Thomas')).toBe('JT');
	});

	it('strips parenthetical qualifiers', () => {
		expect(getInitials('Jerry Thomas (Admin)')).toBe('JT');
	});

	it('caps at two letters', () => {
		expect(getInitials('Ada Belle Curie')).toBe('AB');
	});

	it('handles a single-word name', () => {
		expect(getInitials('alice')).toBe('A');
	});

	it('falls back to "You" for null/undefined', () => {
		expect(getInitials(undefined)).toBe('Y');
		expect(getInitials(null)).toBe('Y');
	});

	it('honours a custom fallback', () => {
		expect(getInitials(undefined, 'Sensei')).toBe('S');
	});

	it('skips words that do not start alphanumeric', () => {
		expect(getInitials('— Jerry')).toBe('J');
	});
});
