import { describe, expect, it, beforeEach, afterEach, vi } from 'vitest';
import { loadGlyphMode, navMark, setGlyphMode, glyphs } from './glyphs.svelte';

// #133: the dōjō honours the same kanji/icon choice as the app. It is a separate
// web app, so the choice lives in the browser (like its colour mode); the
// default is kanji, as in the app — every dōjō mark follows the choice now, so
// a default of icons would turn all of them into icons for every user.
describe('dōjō glyph mode', () => {
	// An in-memory store: Node's own experimental localStorage global shadows
	// jsdom's here and has no clear().
	beforeEach(() => {
		const m = new Map<string, string>();
		vi.stubGlobal('localStorage', {
			getItem: (k: string) => m.get(k) ?? null,
			setItem: (k: string, v: string) => void m.set(k, v)
		});
	});
	afterEach(() => vi.unstubAllGlobals());

	it('defaults to kanji and reads a persisted choice', () => {
		expect(loadGlyphMode()).toBe('kanji');
		localStorage.setItem('dojo-glyphs', 'icon');
		expect(loadGlyphMode()).toBe('icon');
		localStorage.setItem('dojo-glyphs', 'emoji');
		expect(loadGlyphMode()).toBe('kanji');
	});

	it('persists a choice and exposes it', () => {
		setGlyphMode('kanji');
		expect(glyphs.mode).toBe('kanji');
		expect(localStorage.getItem('dojo-glyphs')).toBe('kanji');
	});
});

describe('navMark', () => {
	const both = { id: 'x', label: 'X', icon: 'inbox', kanji: '受' };
	it('shows the mark the mode asks for', () => {
		expect(navMark(both, 'icon')).toEqual({ kind: 'icon', value: 'inbox' });
		expect(navMark(both, 'kanji')).toEqual({ kind: 'kanji', value: '受' });
	});
	it('falls back to the other mark rather than an empty slot', () => {
		expect(navMark({ id: 'x', label: 'X', kanji: '受' }, 'icon')).toEqual({ kind: 'kanji', value: '受' });
		expect(navMark({ id: 'x', label: 'X', icon: 'inbox' }, 'kanji')).toEqual({ kind: 'icon', value: 'inbox' });
	});
});
