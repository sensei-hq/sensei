import { describe, expect, it, beforeEach, afterEach, vi } from 'vitest';
import { loadGlyphMode, loadGlyphChoice, navMark, setGlyphMode, syncGlyphs, chooseGlyphMode, glyphs } from './glyphs.svelte';

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

	it('remembers when a choice was made', () => {
		setGlyphMode('icon', new Date('2026-10-27T10:00:00Z'));
		expect(loadGlyphChoice()).toEqual({ glyphs: 'icon', glyphs_set_at: '2026-10-27T10:00:00.000Z' });
	});

	// #133: the choice syncs with the app through the dōjō — the later one wins.
	describe('sync', () => {
		const answer = (body: unknown, ok = true) =>
			Promise.resolve({ ok, status: ok ? 200 : 503, json: async () => body } as Response);

		it('takes the dōjō’s later choice', async () => {
			setGlyphMode('kanji', new Date('2026-10-27T09:00:00Z'));
			const f = vi.fn(() => answer({ glyphs: 'icon', glyphs_set_at: '2026-10-27T10:00:00Z' }));
			expect(await syncGlyphs(f as never)).toBeNull();
			expect([glyphs.mode, loadGlyphChoice().glyphs_set_at]).toEqual(['icon', '2026-10-27T10:00:00Z']);
			expect(f).toHaveBeenCalledTimes(1);
		});

		it('sends its own later choice', async () => {
			setGlyphMode('icon', new Date('2026-10-27T11:00:00Z'));
			const f = vi.fn((_url: string, init?: RequestInit) =>
				init?.method === 'PUT' ? answer(JSON.parse(String(init.body))) : answer({ glyphs: 'kanji', glyphs_set_at: null })
			);
			expect(await syncGlyphs(f as never)).toBeNull();
			const put = f.mock.calls.find(([, i]) => i?.method === 'PUT')!;
			expect(JSON.parse(String(put[1]!.body))).toEqual({ glyphs: 'icon', glyphs_set_at: '2026-10-27T11:00:00.000Z' });
			expect(glyphs.mode).toBe('icon');
		});

		it('says why it could not sync, and keeps the local choice', async () => {
			setGlyphMode('icon', new Date('2026-10-27T11:00:00Z'));
			expect(await syncGlyphs((() => answer({}, false)) as never)).toMatch(/503/);
			expect(glyphs.mode).toBe('icon');
		});

		it('sends nothing when neither side has chosen', async () => {
			const f = vi.fn(() => answer({ glyphs: 'kanji', glyphs_set_at: null }));
			expect(await syncGlyphs(f as never)).toBeNull();
			expect(f).toHaveBeenCalledTimes(1);
		});

		it('says why when the dōjō cannot be reached, instead of throwing', async () => {
			const down = vi.fn(() => Promise.reject(new TypeError('Failed to fetch')));
			expect(await syncGlyphs(down as never)).toMatch(/Failed to fetch/);
			expect(await chooseGlyphMode('icon', down as never)).toMatch(/Failed to fetch/);
			expect(glyphs.mode).toBe('icon');
		});

		it('choosing sends the choice', async () => {
			const f = vi.fn((_u: string, init?: RequestInit) => answer(JSON.parse(String(init?.body))));
			await chooseGlyphMode('icon', f as never);
			expect(f).toHaveBeenCalledWith('/v1/you/preferences', expect.objectContaining({ method: 'PUT' }));
			expect(glyphs.mode).toBe('icon');
		});
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
