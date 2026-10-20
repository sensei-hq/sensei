// app/src/lib/glyphs.ts

import { getContext, setContext } from 'svelte';
//
// Kanji or icons across the app (#133): one preference, and the one place the
// rokkit `fields` mapping is derived from it, so no component decides for
// itself. Kanji is part of the product's voice; icon mode is an option.
//
// rokkit's ItemContent renders an `i-*` class as an icon and anything else as
// literal text, so pointing the List's `icon` slot at `kanji` or at `icon` is
// the whole switch. It does NOT fall back: an entry with no `icon` would render
// an empty slot in icon mode, which is why every list carries both values and
// its spec asserts it.

export type GlyphMode = 'kanji' | 'icon';

/** The choices Settings → General offers (label → wire value). */
export const GLYPH_OPTIONS: { label: string; value: GlyphMode }[] = [
  { label: 'Kanji', value: 'kanji' },
  { label: 'Icons', value: 'icon' },
];

/** The daemon config key the preference is stored under. */
export const GLYPHS_KEY = 'ui.glyphs';

/** The mode a config map says, kanji unless it says `icon`. */
export function glyphMode(config: Record<string, string>): GlyphMode {
  return config[GLYPHS_KEY] === 'icon' ? 'icon' : 'kanji';
}

/** The rokkit List `fields` for a mode: which property fills the icon slot. */
export function glyphFields(mode: GlyphMode): { icon: string; label: string } {
  return { icon: mode, label: 'text' };
}

/** Whether `icon` is an installed Solar Bold Duotone class — the only icon
 *  style the design system allows (§ Iconography). `solar` is the set of icon
 *  names in `@iconify-json/solar`. */
export function isSolarBoldDuotone(icon: string, solar: Set<string>): boolean {
  const m = /^i-solar-([a-z0-9-]+-bold-duotone)$/.exec(icon);
  return m !== null && solar.has(m[1]);
}

/** One mark, resolved for a mode (#133). */
export type Glyph = { kind: 'kanji' | 'icon'; value: string };

/** The mark to show for an entry outside a rokkit List. */
export function glyphOf(entry: { kanji: string; icon?: string }, mode: GlyphMode): Glyph {
  return mode === 'icon' && entry.icon
    ? { kind: 'icon', value: entry.icon }
    : { kind: 'kanji', value: entry.kanji };
}

const GLYPH_CONTEXT = Symbol('glyph-mode');

/** Provide the glyph mode to every component below (#133). The root layout
 *  calls this once with a getter over the config, so a mark follows the
 *  preference without each screen passing it down. */
export function setGlyphMode(get: () => GlyphMode): void {
  setContext(GLYPH_CONTEXT, get);
}

/** The mode provided above, or kanji when nothing provides one. Call during
 *  component init; the returned getter stays reactive. */
export function useGlyphMode(): () => GlyphMode {
  return getContext<(() => GlyphMode) | undefined>(GLYPH_CONTEXT) ?? (() => 'kanji');
}
