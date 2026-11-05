// packages/ui/glyph-mode.js — kanji or icons, provided once from the root (#133).
//
// One preference decides every mark: kanji mode shows kanji, icon mode shows
// icons (user, 2026-10-27). The root layout of each app provides a getter over
// its own store — the desktop app's daemon config, the dōjō's persisted choice —
// and every shared component below reads it from here, so a component never
// decides for itself and the two apps cannot disagree.
//
// `Symbol.for`, not `Symbol`: the key must be the same even if a bundler ever
// loads this module twice (once through `@sensei/ui`, once by path).

import { getContext, setContext } from 'svelte';

/** @typedef {'kanji' | 'icon'} GlyphMode */

const GLYPH_CONTEXT = Symbol.for('sensei.glyph-mode');

/**
 * Provide the glyph mode to every component below. Call once, during the root
 * layout's init, with a getter so marks follow a change without a reload.
 * @param {() => GlyphMode} get
 * @returns {void}
 */
export function setGlyphMode(get) {
  setContext(GLYPH_CONTEXT, get);
}

/**
 * The mode provided above, or kanji when nothing provides one — kanji is the
 * product's voice; icons are the option. Call during component init; the
 * returned getter stays reactive.
 * @returns {() => GlyphMode}
 */
export function useGlyphMode() {
  /** @type {(() => GlyphMode) | undefined} */
  const get = getContext(GLYPH_CONTEXT);
  return get ?? (() => 'kanji');
}
