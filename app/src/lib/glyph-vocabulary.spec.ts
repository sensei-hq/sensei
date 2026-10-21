import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'fs';
import { join } from 'path';
import { isSolarBoldDuotone } from './glyphs';
import { IDENTITY_KANJI, KANJI_ICONS, iconForKanji } from './glyph-vocabulary';

// #133: sensei's kanji are a vocabulary — 連 is "connect" wherever it appears —
// so one table gives each its icon. Every kanji the app renders must be in it,
// or be declared an identity mark that stays kanji in both modes.
const solar = new Set(
  Object.keys(JSON.parse(readFileSync('node_modules/@iconify-json/solar/icons.json', 'utf8')).icons),
);

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) return sources(p);
    return /\.(svelte|ts)$/.test(p) && !p.includes('.spec.') ? [p] : [];
  });
}

/** Kanji in a kanji position: a `kanji` field or prop, a `char=` prop, or the
 *  text of a `class="kanji…"` span. */
const KANJI_SITE = /(?:kanji["']?\s*[:=]\s*["'{]?\s*["']?|class="kanji[^"]*">\s*|char=")(\p{Script=Han})/gu;

describe('KANJI_ICONS', () => {
  it('maps every kanji to an installed Solar Bold Duotone icon', () => {
    expect(Object.keys(KANJI_ICONS).length).toBeGreaterThan(100);
    for (const [k, icon] of Object.entries(KANJI_ICONS)) {
      expect(isSolarBoldDuotone(icon, solar), `${k}: ${icon}`).toBe(true);
    }
  });

  it('covers every kanji the app renders, except identity marks', () => {
    const missing = new Map<string, string>();
    for (const file of sources('src')) {
      for (const m of readFileSync(file, 'utf8').matchAll(KANJI_SITE)) {
        const k = m[1];
        if (!KANJI_ICONS[k] && !IDENTITY_KANJI.has(k)) missing.set(k, file.replace('src/', ''));
      }
    }
    expect(Object.fromEntries(missing)).toEqual({});
  });

  it('gives no icon to an identity mark or an unknown kanji', () => {
    expect(iconForKanji('連')).toBe('i-solar-link-round-bold-duotone');
    for (const k of IDENTITY_KANJI) expect(iconForKanji(k)).toBeUndefined();
    expect(iconForKanji('鬱')).toBeUndefined();
  });
});
