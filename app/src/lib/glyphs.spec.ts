import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { GLYPH_OPTIONS, GLYPHS_KEY, glyphFields, glyphMode, glyphOf, isSolarBoldDuotone } from './glyphs';

// #133: one preference switches the whole app between kanji and icons, and
// `fields` is derived from it here alone — no component decides for itself.
describe('glyphMode', () => {
  it('is kanji unless the preference says icon', () => {
    expect(glyphMode({})).toBe('kanji');
    expect(glyphMode({ [GLYPHS_KEY]: 'icon' })).toBe('icon');
    expect(glyphMode({ [GLYPHS_KEY]: 'kanji' })).toBe('kanji');
    expect(glyphMode({ [GLYPHS_KEY]: 'emoji' })).toBe('kanji');
  });
});

describe('GLYPH_OPTIONS', () => {
  it('offers exactly the two modes, kanji first', () => {
    expect(GLYPH_OPTIONS.map((o) => o.value)).toEqual(['kanji', 'icon']);
    for (const o of GLYPH_OPTIONS) expect(glyphMode({ [GLYPHS_KEY]: o.value })).toBe(o.value);
  });
});

describe('glyphFields', () => {
  it('points the List icon slot at the kanji or the icon', () => {
    expect(glyphFields('kanji')).toEqual({ icon: 'kanji', label: 'text' });
    expect(glyphFields('icon')).toEqual({ icon: 'icon', label: 'text' });
  });
});

describe('isSolarBoldDuotone', () => {
  const solar = new Set(
    Object.keys(JSON.parse(readFileSync('node_modules/@iconify-json/solar/icons.json', 'utf8')).icons)
  );
  it('accepts only an installed Solar Bold Duotone class', () => {
    expect(isSolarBoldDuotone('i-solar-home-smile-bold-duotone', solar)).toBe(true);
    expect(isSolarBoldDuotone('i-solar-home-smile-linear', solar)).toBe(false);
    expect(isSolarBoldDuotone('i-solar-no-such-thing-bold-duotone', solar)).toBe(false);
    expect(isSolarBoldDuotone('家', solar)).toBe(false);
  });
});

// #133: for marks rendered outside a rokkit List (role chips, project glyphs),
// the same rule in one function — and a missing icon falls back to the kanji,
// never to an empty slot.
describe('glyphOf', () => {
  const both = { kanji: '後', icon: 'i-solar-server-bold-duotone' };
  it('picks the kanji or the icon by mode', () => {
    expect(glyphOf(both, 'kanji')).toEqual({ kind: 'kanji', value: '後' });
    expect(glyphOf(both, 'icon')).toEqual({ kind: 'icon', value: 'i-solar-server-bold-duotone' });
  });
  it('falls back to the kanji when there is no icon', () => {
    expect(glyphOf({ kanji: '禅' }, 'icon')).toEqual({ kind: 'kanji', value: '禅' });
  });
});
