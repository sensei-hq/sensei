import { describe, expect, it } from 'vitest';
import { needsTone, needsActions } from './needs';

// The needs-you per-kind maps decide the icon + action set for each kind of
// blocked item. They're pure, so unit-test them directly (the components render
// through them).
describe('needs per-kind maps', () => {
	it('needsTone maps each kind to its icon + verb', () => {
		expect(needsTone('gate').label).toBe('approve');
		expect(needsTone('conflict').icon).toBe('danger-triangle');
		expect(needsTone('decision').label).toBe('decide');
		expect(needsTone('review').label).toBe('review');
	});

	it('needsTone falls back to decision for an unknown kind', () => {
		expect(needsTone('mystery')).toEqual(needsTone('decision'));
		expect(needsTone(undefined)).toEqual(needsTone('decision'));
	});

	it('needsActions gives gate an approve + deny set', () => {
		const a = needsActions('gate');
		expect(a.map((x) => x.id)).toEqual(['approve', 'deny']);
		expect(a[0].primary).toBe(true);
	});

	it('needsActions gives conflict a single settle, decision a single decide', () => {
		expect(needsActions('conflict').map((x) => x.id)).toEqual(['settle']);
		expect(needsActions('decision').map((x) => x.id)).toEqual(['decide']);
	});

	it('needsActions gives review an approve + decline set', () => {
		const a = needsActions('review');
		expect(a.map((x) => x.id)).toEqual(['approve', 'deny']);
		expect(a[1].label).toBe('Decline');
	});

	it('needsActions falls back to decision for an unknown kind', () => {
		expect(needsActions('mystery')).toEqual(needsActions('decision'));
	});
});
