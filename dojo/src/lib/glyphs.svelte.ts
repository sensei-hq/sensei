// The dōjō's kanji/icon choice (#133) — the same preference as the app's
// Settings → General → Glyphs. It lives in the browser, as the colour mode does
// (theme.svelte.ts), with when it was chosen, and syncs through the dōjō
// (/v1/you/preferences), where the app's daemon syncs it too: the later choice
// wins (#133). The default is kanji, as in the app.

import type { KitNavItem } from './components/kit/types';
import { laterChoice, type GlyphMode, type GlyphPref } from './preferences';

export type { GlyphMode };

const KEY = 'dojo-glyphs';
const KEY_AT = 'dojo-glyphs-at';
const PREFERENCES = '/v1/you/preferences';

/** The choices the account menu offers (label → value). */
export const GLYPH_OPTIONS: { label: string; value: GlyphMode }[] = [
	{ label: 'Icons', value: 'icon' },
	{ label: 'Kanji', value: 'kanji' }
];

/** The current choice; components read `glyphs.mode`. */
export const glyphs = $state<{ mode: GlyphMode }>({ mode: 'kanji' });

/** The persisted choice, kanji unless it says icon. */
export function loadGlyphMode(): GlyphMode {
	if (typeof localStorage === 'undefined') return 'kanji';
	return localStorage.getItem(KEY) === 'icon' ? 'icon' : 'kanji';
}

/** The persisted choice and when it was made (null: never chosen here). */
export function loadGlyphChoice(): GlyphPref {
	const at = typeof localStorage === 'undefined' ? null : localStorage.getItem(KEY_AT);
	return { glyphs: loadGlyphMode(), glyphs_set_at: at };
}

function keep(choice: GlyphPref): void {
	glyphs.mode = choice.glyphs;
	if (typeof localStorage === 'undefined') return;
	localStorage.setItem(KEY, choice.glyphs);
	if (choice.glyphs_set_at) localStorage.setItem(KEY_AT, choice.glyphs_set_at);
}

/** Set and persist the choice, made now (or at `at`). */
export function setGlyphMode(mode: GlyphMode, at: Date = new Date()): void {
	keep({ glyphs: mode, glyphs_set_at: at.toISOString() });
}

/** Choose, and send the choice to the dōjō. A failed send leaves the choice
 *  here; the next sync carries it. Returns why it could not send, or null. */
export async function chooseGlyphMode(mode: GlyphMode, f: typeof fetch = fetch): Promise<string | null> {
	setGlyphMode(mode);
	return reach(() => send(f));
}

/** A fetch that throws (offline) is said, not thrown. */
async function reach(run: () => Promise<string | null>): Promise<string | null> {
	try {
		return await run();
	} catch (e) {
		return `Could not reach the dōjō: ${e instanceof Error ? e.message : String(e)}`;
	}
}

async function send(f: typeof fetch): Promise<string | null> {
	const res = await f(PREFERENCES, {
		method: 'PUT',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(loadGlyphChoice())
	});
	if (!res.ok) return `Could not save the glyph choice (${res.status})`;
	keep(laterChoice(loadGlyphChoice(), (await res.json()) as GlyphPref));
	return null;
}

/** Bring this browser and the dōjō to the later choice: take the dōjō's when it
 *  is later, send this one when it is. Returns why it could not, or null. */
export async function syncGlyphs(f: typeof fetch = fetch): Promise<string | null> {
	return reach(() => pull(f));
}

async function pull(f: typeof fetch): Promise<string | null> {
	const res = await f(PREFERENCES);
	if (!res.ok) return `Could not read the glyph choice (${res.status})`;
	const remote = (await res.json()) as GlyphPref;
	const local = loadGlyphChoice();
	const later = laterChoice(remote, local);
	if (later === remote) {
		keep(remote);
		return null;
	}
	if (!local.glyphs_set_at) return null; // neither side has chosen: nothing to send
	const sent = await f(PREFERENCES, {
		method: 'PUT',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(local)
	});
	return sent.ok ? null : `Could not save the glyph choice (${sent.status})`;
}

/** The mark a nav item shows for a mode, falling back to its other mark rather
 *  than an empty slot. */
export function navMark(it: KitNavItem, mode: GlyphMode): { kind: 'kanji' | 'icon'; value: string } {
	const icon = it.icon ? { kind: 'icon' as const, value: it.icon } : null;
	const kanji = it.kanji ? { kind: 'kanji' as const, value: it.kanji } : null;
	return (mode === 'icon' ? (icon ?? kanji) : (kanji ?? icon)) ?? { kind: 'kanji', value: '' };
}
