// The dōjō's kanji/icon choice (#133) — the same preference as the app's
// Settings → General → Glyphs. The dōjō is a separate web app with no reach into
// the daemon's config, so the choice lives in the browser, as the colour mode
// does (theme.svelte.ts). The default is icons: what the dōjō nav has always
// shown, so nothing changes until someone picks kanji.

import type { KitNavItem } from './components/kit/types';

export type GlyphMode = 'kanji' | 'icon';

const KEY = 'dojo-glyphs';

/** The choices the account menu offers (label → value). */
export const GLYPH_OPTIONS: { label: string; value: GlyphMode }[] = [
	{ label: 'Icons', value: 'icon' },
	{ label: 'Kanji', value: 'kanji' }
];

/** The current choice; components read `glyphs.mode`. */
export const glyphs = $state<{ mode: GlyphMode }>({ mode: 'icon' });

/** The persisted choice, icons unless it says kanji. */
export function loadGlyphMode(): GlyphMode {
	if (typeof localStorage === 'undefined') return 'icon';
	return localStorage.getItem(KEY) === 'kanji' ? 'kanji' : 'icon';
}

/** Set and persist the choice. */
export function setGlyphMode(mode: GlyphMode): void {
	glyphs.mode = mode;
	if (typeof localStorage !== 'undefined') localStorage.setItem(KEY, mode);
}

/** The mark a nav item shows for a mode, falling back to its other mark rather
 *  than an empty slot. */
export function navMark(it: KitNavItem, mode: GlyphMode): { kind: 'kanji' | 'icon'; value: string } {
	const icon = it.icon ? { kind: 'icon' as const, value: it.icon } : null;
	const kanji = it.kanji ? { kind: 'kanji' as const, value: it.kanji } : null;
	return (mode === 'icon' ? (icon ?? kanji) : (kanji ?? icon)) ?? { kind: 'kanji', value: '' };
}
