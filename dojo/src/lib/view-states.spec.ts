// billing build (Design) — D87 remembered panels for the dōjō. The shell
// stores nothing; the dōjō keeps every view's panels in one map, keyed by view id,
// in localStorage `sensei.dojo.views` (CONTRACT §4). The first visit opens
// everything (an empty state — a missing key is open); later visits restore it.
// A failed or missing read is a first visit, never an error on the screen.
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import * as v from './view-states.svelte';

describe('dōjō view states (D87)', () => {
	// An in-memory store, as glyphs.spec.ts does: Node's own experimental
	// localStorage global shadows jsdom's here and has no clear().
	let store: Map<string, string>;
	beforeEach(() => {
		store = new Map<string, string>();
		vi.stubGlobal('localStorage', {
			getItem: (k: string) => store.get(k) ?? null,
			setItem: (k: string, v: string) => void store.set(k, v)
		});
	});
	afterEach(() => vi.unstubAllGlobals());

	it('the first visit of a view is everything open', () => {
		expect(v.STORAGE_KEY).toBe('sensei.dojo.views');
		expect(v.panelsFor('dojo.org.billing')).toEqual({});
	});

	it('a later visit restores what the user left, per view id', () => {
		v.savePanels('dojo.org.billing', { info: false });
		expect(v.panelsFor('dojo.org.billing')).toEqual({ info: false });
		expect(v.panelsFor('dojo.org.members')).toEqual({});
		expect(JSON.parse(store.get('sensei.dojo.views')!)).toEqual({ 'dojo.org.billing': { info: false } });
	});

	it('unreadable storage reads as a first visit', () => {
		store.set('sensei.dojo.views', '{not json');
		expect(v.panelsFor('dojo.org.billing')).toEqual({});
	});

	// CONTRACT §4: "a failed or missing read means first visit" — a private window's
	// accessor throws; that is a first visit too, never an error.
	it('a read that throws reads as a first visit', () => {
		vi.stubGlobal('localStorage', {
			getItem: () => {
				throw new DOMException('denied', 'SecurityError');
			},
			setItem: () => {
				throw new DOMException('denied', 'SecurityError');
			}
		});
		expect(() => v.panelsFor('dojo.org.billing')).not.toThrow();
		expect(v.panelsFor('dojo.org.billing')).toEqual({});
	});
});
